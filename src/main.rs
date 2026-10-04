use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use clap::{Parser as ClapParser, Subcommand};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use pulldown_cmark::{Options, Parser, html};
use std::{
    fmt::Write,
    fs, io,
    path::{Component, Path as FsPath, PathBuf},
    sync::Arc,
    time::Instant,
};
use walkdir::WalkDir;

const CSS: &str = include_str!("style.css");
type Root = Arc<PathBuf>;
mod export;

#[derive(ClapParser)]
#[command(version, about)]
struct Cli {
    /// Address and port to listen on
    #[arg(long, default_value = "127.0.0.1:5779", value_name = "HOST:PORT")]
    addr: String,

    /// Directory to read Markdown from (relative to the working directory)
    #[arg(long, global = true, default_value = ".", value_name = "PATH")]
    dir: PathBuf,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Export the collection as a static website without starting a server
    Export {
        /// New directory to write the website to
        #[arg(long, default_value = "markhaus-site", value_name = "PATH")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let root = fs::canonicalize(&cli.dir)?;
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Markdown path must be a directory: {}", root.display()),
        )
        .into());
    }
    if let Some(Command::Export { output }) = cli.command {
        let count = export::write_site(&root, &output)?;
        println!(
            "Exported {count} documents to {}",
            fs::canonicalize(output)?.display()
        );
        return Ok(());
    }
    let listener = tokio::net::TcpListener::bind(&cli.addr).await?;
    println!("Markhaus → http://{}", listener.local_addr()?);
    println!("Reading Markdown from {}", root.display());
    axum::serve(listener, app(root))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

fn app(root: PathBuf) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/read/{*file}", get(document))
        .route("/style.css", get(stylesheet))
        .fallback(|| async { AppError::NotFound })
        .layer(middleware::map_response(response_headers))
        .with_state(Arc::new(root))
}

async fn response_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; style-src 'self'; img-src https: http: data:; base-uri 'none'; frame-ancestors 'none'; form-action 'none'",
        ),
    );
    response
}

async fn stylesheet() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], CSS)
}

async fn index(State(root): State<Root>) -> Result<Html<String>, AppError> {
    let started = Instant::now();
    let files = tokio::task::spawn_blocking(move || discover(&root))
        .await
        .map_err(AppError::worker)??;
    Ok(Html(index_page(&files, started, false)))
}

fn index_page(files: &[String], started: Instant, static_site: bool) -> String {
    let count = files.len();
    let mut rows = String::new();
    for (number, file) in files.iter().enumerate() {
        let path = escape(file);
        let href = if static_site {
            format!("read/{}.html", encoded_path(file))
        } else {
            document_url(file)
        };
        write!(rows, r#"<li><a class="file-row" href="{href}"><span class="file-number">{:02}</span><span class="file-name">{path}</span><span class="file-kind">Markdown</span><span class="file-arrow" aria-hidden="true">↗</span></a></li>"#, number + 1).unwrap();
    }
    if files.is_empty() {
        let message = if static_site {
            "No Markdown documents were found in the source directory."
        } else {
            "Add a .md or .markdown file to this directory, then refresh to start reading."
        };
        write!(rows, r#"<li class="empty"><span class="empty-symbol" aria-hidden="true">＋</span><h3>A blank canvas.</h3><p>{message}</p></li>"#).unwrap();
    }
    let noun = if count == 1 { "document" } else { "documents" };
    let body = format!(
        r#"<section class="hero" aria-labelledby="hero-title">
          <div class="hero-copy"><p class="eyebrow">A little order for your ideas</p><h1 id="hero-title">Words.<br>In good<br><span>form.</span></h1><p class="hero-description">Your Markdown, beautifully readable.<br>Pick a document and make yourself at home.</p></div>
          <div class="composition" aria-hidden="true"><span class="composition-label">FORM / FUNCTION</span><div class="circle"></div><div class="square"></div><div class="triangle"></div><span class="composition-caption">THE READING ROOM — № 01</span></div>
        </section>
        <section class="library" aria-labelledby="library-title"><div class="section-heading"><h2 id="library-title">The collection<span class="red-dot" aria-hidden="true"></span></h2><span class="count">{count:02} {noun}</span></div><ol class="file-list">{rows}</ol></section>"#,
    );
    page("The collection", &body, started, static_site.then_some(""))
}

async fn document(
    State(root): State<Root>,
    Path(file): Path<String>,
) -> Result<Html<String>, AppError> {
    let started = Instant::now();
    let (title, rendered) = tokio::task::spawn_blocking(move || {
        let path = resolve_document(&root, &file)?;
        let markdown = fs::read_to_string(path).map_err(AppError::from_io)?;
        Ok::<_, AppError>((file, render_markdown(&markdown)))
    })
    .await
    .map_err(AppError::worker)??;
    Ok(Html(document_page(&title, &rendered, started, None)))
}

fn document_page(
    title: &str,
    rendered: &str,
    started: Instant,
    root_prefix: Option<&str>,
) -> String {
    let escaped_title = escape(title);
    let home = home_url(root_prefix);
    let body = format!(
        r#"<nav class="reader-nav" aria-label="Document navigation"><a class="back-link" href="{home}">← The collection</a><span class="reader-label">THE READING ROOM</span></nav><section class="document-heading"><p class="eyebrow">Markdown / Document</p><h1>{escaped_title}</h1><div class="reading-rule" aria-hidden="true"><span></span><span></span><span></span></div></section><article class="prose" aria-label="{escaped_title}">{rendered}</article>"#,
    );
    page(title, &body, started, root_prefix)
}

fn discover(root: &FsPath) -> Result<Vec<String>, AppError> {
    let mut files = Vec::new();
    let entries = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !excluded(
                    &entry.file_name().to_string_lossy(),
                    entry.file_type().is_dir(),
                )
        });
    for entry in entries {
        let entry = entry.map_err(|error| {
            eprintln!("Cannot scan Markdown directory: {error}");
            AppError::Internal
        })?;
        if entry.file_type().is_file() && is_markdown(entry.path()) {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| AppError::Internal)?;
            if let Some(path) = relative.to_str() {
                // URL paths use forward slashes on every platform.
                files.push(path.replace(std::path::MAIN_SEPARATOR, "/"));
            }
        }
    }
    files.sort();
    Ok(files)
}

fn excluded(name: &str, is_dir: bool) -> bool {
    name.starts_with('.') || (is_dir && matches!(name, "target" | "node_modules"))
}

fn is_markdown(path: &FsPath) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

fn resolve_document(root: &FsPath, file: &str) -> Result<PathBuf, AppError> {
    let relative = FsPath::new(file);
    if !is_markdown(relative) {
        return Err(AppError::NotFound);
    }
    let mut path = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(AppError::NotFound);
        };
        path.push(name);
        let metadata = fs::symlink_metadata(&path).map_err(AppError::from_io)?;
        if metadata.file_type().is_symlink() || excluded(&name.to_string_lossy(), metadata.is_dir())
        {
            return Err(AppError::NotFound);
        }
    }
    let canonical = fs::canonicalize(&path).map_err(AppError::from_io)?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(AppError::NotFound);
    }
    Ok(canonical)
}

fn render_markdown(markdown: &str) -> String {
    render_markdown_with_links(markdown, None)
}

fn render_markdown_with_links(markdown: &str, root_prefix: Option<&str>) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let mut rendered = String::new();
    let events = Parser::new_ext(markdown, options).map(|mut event| {
        if let Some(prefix) = root_prefix
            && let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { dest_url, .. }) =
                &mut event
            && let Some(url) = export::document_link(dest_url, prefix)
        {
            *dest_url = url.into();
        }
        event
    });
    html::push_html(&mut rendered, events);
    // Preserve useful document HTML without allowing scripts or event handlers.
    ammonia::Builder::default()
        .add_generic_attributes(&["id"])
        .add_tag_attributes("div", &["class"])
        .add_tags(&["input"])
        .add_tag_attributes("input", &["type", "checked", "disabled"])
        .set_tag_attribute_value("input", "type", "checkbox")
        .set_tag_attribute_value("input", "disabled", "")
        .clean(&rendered)
        .to_string()
}

fn document_url(file: &str) -> String {
    format!("/read/{}", encoded_path(file))
}

fn encoded_path(file: &str) -> String {
    file.split('/')
        .map(|part| utf8_percent_encode(part, NON_ALPHANUMERIC).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }
    output
}

fn home_url(root_prefix: Option<&str>) -> String {
    root_prefix.map_or_else(|| "/".into(), |prefix| format!("{prefix}index.html"))
}

fn page(title: &str, body: &str, started: Instant, root_prefix: Option<&str>) -> String {
    let title = escape(title);
    let home = home_url(root_prefix);
    let stylesheet = root_prefix.map_or_else(
        || "/style.css".into(),
        |prefix| format!("{prefix}style.css"),
    );
    let mut output = format!(
        r##"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta name="color-scheme" content="light"><title>{title} — Markhaus</title><link rel="icon" href="data:,"><link rel="stylesheet" href="{stylesheet}"></head><body><a class="skip-link" href="#main">Skip to content</a><div class="shell"><header class="site-header"><a class="brand" href="{home}" aria-label="Markhaus home"><span class="brand-mark" aria-hidden="true"></span>markhaus<span class="brand-period">.</span></a><span class="header-note">A SPACE FOR WORDS<span class="header-shapes" aria-hidden="true"><i></i><i></i><i></i></span></span></header><main id="main">{body}</main><footer class="site-footer"><span>Simple files. Considered form."##,
    );
    // Measure after the document body and HTML shell have been generated.
    let milliseconds = started.elapsed().as_secs_f64() * 1000.0;
    write!(output, r#"<span class="generation-time">Generated in {milliseconds:.2} ms</span></span><span class="footer-signature">MARKHAUS <span aria-hidden="true">↗</span></span></footer></div></body></html>"#).unwrap();
    output
}

enum AppError {
    NotFound,
    Internal,
}

impl AppError {
    fn from_io(error: io::Error) -> Self {
        if matches!(
            error.kind(),
            io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
        ) {
            Self::NotFound
        } else {
            eprintln!("Cannot read Markdown: {error}");
            Self::Internal
        }
    }

    fn worker(error: tokio::task::JoinError) -> Self {
        eprintln!("Markdown worker failed: {error}");
        Self::Internal
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, title, message) = match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "404",
                "This document isn't in the collection.",
            ),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong",
                "We couldn't read the collection. Check the server output for details.",
            ),
        };
        let body = format!(
            r#"<section class="error-page"><p class="eyebrow">A small interruption</p><h1>{title}</h1><p>{message}</p><a class="back-link" href="/">← Back to the collection</a></section>"#
        );
        (status, Html(page(title, &body, Instant::now(), None))).into_response()
    }
}

#[cfg(test)]
mod tests;
