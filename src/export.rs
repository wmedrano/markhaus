use super::*;
use percent_encoding::percent_decode_str;

pub(super) fn write_site(root: &FsPath, output: &FsPath) -> io::Result<usize> {
    if output.try_exists()? {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "Export directory already exists: {}. Choose a new --output directory.",
                output.display()
            ),
        ));
    }

    let started = Instant::now();
    let files =
        discover(root).map_err(|_| io::Error::other("Cannot discover Markdown documents"))?;
    let index = index_page(&files, started, true);
    // Prepare all pages before writing so an unreadable document leaves no partial export.
    let mut pages = Vec::with_capacity(files.len());
    for file in &files {
        let started = Instant::now();
        let path = resolve_document(root, file)
            .map_err(|_| io::Error::other(format!("Cannot resolve Markdown document: {file}")))?;
        let markdown = fs::read_to_string(path)?;
        let prefix = "../".repeat(file.split('/').count());
        let rendered = render_markdown_with_links(&markdown, Some(&prefix));
        pages.push((file, document_page(file, &rendered, started, Some(&prefix))));
    }

    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(output)?;
    fs::write(output.join("index.html"), index)?;
    fs::write(output.join("style.css"), CSS)?;
    for (file, page) in pages {
        // Keep the original extension to distinguish e.g. notes.md and notes.markdown.
        let destination = output.join("read").join(format!("{file}.html"));
        fs::create_dir_all(destination.parent().unwrap())?;
        fs::write(destination, page)?;
    }
    Ok(files.len())
}

/// Rewrite local Markdown links while retaining query strings and fragments.
pub(super) fn document_link(url: &str, root_prefix: &str) -> Option<String> {
    let end = url.find(['?', '#']).unwrap_or(url.len());
    let (path, suffix) = url.split_at(end);
    if path.is_empty() || path.starts_with("//") || path.contains(':') {
        return None;
    }
    let decoded = percent_decode_str(path).decode_utf8().ok()?;
    if !is_markdown(FsPath::new(decoded.as_ref())) {
        return None;
    }
    if let Some(relative) = path.strip_prefix('/') {
        Some(format!("{root_prefix}read/{relative}.html{suffix}"))
    } else {
        Some(format!("{path}.html{suffix}"))
    }
}
