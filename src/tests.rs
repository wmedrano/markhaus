use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tempfile::TempDir;
use tower::ServiceExt;

async fn request(app: &Router, uri: &str) -> (StatusCode, axum::http::HeaderMap, String) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}

fn fixture() -> (TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let router = app(fs::canonicalize(directory.path()).unwrap());
    (directory, router)
}

#[tokio::test]
async fn homepage_discovers_nested_markdown_and_ignores_other_files() {
    let (directory, app) = fixture();
    for folder in ["notes", ".private", "target", "node_modules"] {
        fs::create_dir(directory.path().join(folder)).unwrap();
    }
    for file in [
        "z.md",
        "notes/a.MARKDOWN",
        ".secret.md",
        ".private/x.md",
        "target/x.md",
        "node_modules/x.md",
        "plain.txt",
    ] {
        fs::write(directory.path().join(file), "# Hello").unwrap();
    }
    let (status, headers, body) = request(&app, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert!(body.contains("02 documents"));
    assert!(body.find("notes/a.MARKDOWN").unwrap() < body.find("z.md").unwrap());
    assert!(!body.contains("secret.md"));
    assert!(!body.contains("x.md"));
    assert!(!body.contains("plain.txt"));
}

#[tokio::test]
async fn discovery_reflects_additions_and_deletions_without_restart() {
    let (directory, app) = fixture();
    assert!(request(&app, "/").await.2.contains("A blank canvas."));
    let path = directory.path().join("fresh.md");
    fs::write(&path, "New document").unwrap();
    assert!(request(&app, "/").await.2.contains("fresh.md"));
    fs::remove_file(path).unwrap();
    assert!(!request(&app, "/").await.2.contains("fresh.md"));
    assert_eq!(
        request(&app, "/read/fresh.md").await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn rendering_reads_edits_on_every_request() {
    let (directory, app) = fixture();
    let path = directory.path().join("live.md");
    fs::write(
        &path,
        "# First\n\n**Bold** and ~~old~~.\n\n| A | B |\n| - | - |\n| 1 | 2 |\n\n- [x] Done\n\nA footnote[^note].\n\n[^note]: Footnote text.\n",
    )
    .unwrap();
    let (status, headers, body) = request(&app, "/read/live.md").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert!(body.contains("<h1>First</h1>"));
    assert!(body.contains("<strong>Bold</strong>"));
    assert!(body.contains("<del>old</del>"));
    assert!(body.contains("<table>"));
    assert!(body.contains("type=\"checkbox\""));
    assert!(body.contains("disabled"));
    assert!(body.contains("href=\"#note\""));
    assert!(body.contains("id=\"note\""));
    fs::write(path, "# Second").unwrap();
    let body = request(&app, "/read/live.md").await.2;
    assert!(body.contains("<h1>Second</h1>"));
    assert!(!body.contains("<h1>First</h1>"));
}

#[tokio::test]
async fn filenames_are_html_escaped_and_url_encoded() {
    let (directory, app) = fixture();
    let name = "café <notes> #1 & 100%.md";
    fs::write(directory.path().join(name), "# Special name").unwrap();
    let body = request(&app, "/").await.2;
    assert!(body.contains("café &lt;notes&gt; #1 &amp; 100%.md"));
    let url = document_url(name);
    assert!(body.contains(&format!("href=\"{url}\"")));
    let (status, _, body) = request(&app, &url).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<h1>Special name</h1>"));
}

#[tokio::test]
async fn rejects_traversal_hidden_files_and_non_markdown() {
    let (directory, app) = fixture();
    fs::write(directory.path().join(".secret.md"), "secret").unwrap();
    fs::write(directory.path().join("password.txt"), "secret").unwrap();
    fs::create_dir(directory.path().join("target")).unwrap();
    fs::write(directory.path().join("target/secret.md"), "secret").unwrap();
    for uri in [
        "/read/%2e%2e/outside.md",
        "/read/%2Fetc%2Foutside.md",
        "/read/.secret.md",
        "/read/password.txt",
        "/read/target/secret.md",
        "/read/missing.md",
        "/unknown",
    ] {
        let (status, headers, _) = request(&app, uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn symlinks_are_neither_discovered_nor_served() {
    use std::os::unix::fs::symlink;
    let (directory, app) = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("outside.md"), "# Outside secret").unwrap();
    symlink(
        outside.path().join("outside.md"),
        directory.path().join("linked.md"),
    )
    .unwrap();
    symlink(outside.path(), directory.path().join("linked-dir")).unwrap();
    assert!(!request(&app, "/").await.2.contains("linked"));
    for uri in ["/read/linked.md", "/read/linked-dir/outside.md"] {
        assert_eq!(request(&app, uri).await.0, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn sanitizes_document_html_and_retains_safe_markdown_links() {
    let (directory, app) = fixture();
    fs::write(directory.path().join("unsafe.md"), "<script>alert('bad')</script>\n\n<img src=\"https://example.com/a.png\" onerror=\"alert('bad')\">\n\n[Unsafe](javascript:alert(1))\n\n[Next](notes/next.md)\n").unwrap();
    let (_, headers, body) = request(&app, "/read/unsafe.md").await;
    assert!(!body.contains("<script"));
    assert!(!body.contains("onerror"));
    assert!(!body.contains("javascript:"));
    assert!(body.contains("href=\"notes/next.md\""));
    assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
}

#[tokio::test]
async fn stylesheet_is_served_with_correct_content_type() {
    let (_, app) = fixture();
    let (status, headers, body) = request(&app, "/style.css").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "text/css; charset=utf-8");
    assert!(body.contains("@media (max-width: 620px)"));
}

#[test]
fn static_export_renders_nested_pages_with_portable_navigation_and_markdown_links() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let output = destination.path().join("site");
    fs::create_dir(source.path().join("notes")).unwrap();
    let special = "café & notes.md";
    fs::write(source.path().join(special), "# Special document").unwrap();
    fs::write(source.path().join("guide.md"), "# Guide").unwrap();
    fs::write(source.path().join("guide.markdown"), "# Alternate guide").unwrap();
    fs::write(source.path().join(".secret.md"), "Secret").unwrap();
    fs::write(source.path().join("notes/nested.MD"), "# Nested\n\n[Guide](../guide.md?mode=read#main)\n\n[Root](/guide.markdown)\n\n[Reference][ref]\n\n[ref]: ../caf%C3%A9%20%26%20notes.md\n\n[External](https://example.com/guide.md)\n\n[Anchor](#main)\n\n<script>bad()</script>").unwrap();
    let root = fs::canonicalize(source.path()).unwrap();
    assert_eq!(export::write_site(&root, &output).unwrap(), 4);

    let index = fs::read_to_string(output.join("index.html")).unwrap();
    assert!(index.contains("04 documents"));
    assert!(index.contains(&format!("href=\"read/{}.html\"", encoded_path(special))));
    assert!(index.contains("href=\"style.css\""));
    assert!(index.contains("href=\"index.html\""));
    assert!(!index.contains(".secret.md"));
    assert_eq!(fs::read_to_string(output.join("style.css")).unwrap(), CSS);
    assert!(output.join("read/guide.md.html").is_file());
    assert!(output.join("read/guide.markdown.html").is_file());
    assert!(
        output
            .join("read")
            .join(format!("{special}.html"))
            .is_file()
    );

    let nested = fs::read_to_string(output.join("read/notes/nested.MD.html")).unwrap();
    assert!(nested.contains("<h1>Nested</h1>"));
    assert!(nested.contains("href=\"../../style.css\""));
    assert!(nested.contains("href=\"../../index.html\""));
    assert!(nested.contains("href=\"../guide.md.html?mode=read#main\""));
    assert!(nested.contains("href=\"../../read/guide.markdown.html\""));
    assert!(nested.contains("href=\"../caf%C3%A9%20%26%20notes.md.html\""));
    assert!(nested.contains("href=\"https://example.com/guide.md\""));
    assert!(nested.contains("href=\"#main\""));
    assert!(nested.contains("Generated in "));
    assert!(!nested.contains("<script"));
}

#[test]
fn static_export_preserves_existing_output_and_rejects_unreadable_markdown() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(source.path()).unwrap();
    fs::write(destination.path().join("index.html"), "Keep this").unwrap();
    let error = export::write_site(&root, destination.path()).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read_to_string(destination.path().join("index.html")).unwrap(),
        "Keep this"
    );

    fs::write(source.path().join("invalid.md"), [0xff, 0xfe]).unwrap();
    let output = destination.path().join("new-site");
    assert!(export::write_site(&root, &output).is_err());
    assert!(!output.exists());
}

#[test]
fn static_export_supports_empty_collections() {
    let source = tempfile::tempdir().unwrap();
    let output = source.path().join("site");
    let root = fs::canonicalize(source.path()).unwrap();
    assert_eq!(export::write_site(&root, &output).unwrap(), 0);
    let index = fs::read_to_string(output.join("index.html")).unwrap();
    assert!(index.contains("00 documents"));
    assert!(index.contains("No Markdown documents were found"));
    assert!(output.join("style.css").is_file());
}

#[test]
fn cli_supports_directory_selection_for_serving_and_export() {
    let defaults = Cli::try_parse_from(["markhaus"]).unwrap();
    assert_eq!(defaults.addr, "127.0.0.1:5779");
    assert_eq!(defaults.dir, PathBuf::from("."));
    assert!(defaults.command.is_none());
    for args in [
        vec![
            "markhaus", "--dir", "my notes", "export", "--output", "site",
        ],
        vec![
            "markhaus", "export", "--dir", "my notes", "--output", "site",
        ],
    ] {
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.dir, PathBuf::from("my notes"));
        assert!(
            matches!(cli.command, Some(Command::Export { output }) if output == FsPath::new("site"))
        );
    }
    assert!(Cli::try_parse_from(["markhaus", "--dir"]).is_err());
    assert!(Cli::try_parse_from(["markhaus", "export", "--output"]).is_err());
}
