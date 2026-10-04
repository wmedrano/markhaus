# Markhaus

A small Rust web server that turns the current working directory into a Markdown reading room, with Bauhaus-inspired typography, geometric forms, and primary colors.

## Run

```sh
cargo run
```

Open **http://127.0.0.1:5779**. Click a document to read it as HTML.

The homepage scans the selected directory and its subdirectories on **every request**. Each document is read and rendered on **every request**, and responses send `Cache-Control: no-store`. Refresh after adding, editing, or deleting a file; no restart is needed.

To read a different directory, use `--dir`:

```sh
cargo run -- --dir /path/to/your/notes
```

By default, `--dir` is `.` (the current working directory). Relative paths are resolved from the working directory. The directory must exist.

To change the listening address:

```sh
cargo run -- --addr 127.0.0.1:8080
```

Run `cargo run -- --help` to see the command-line options.

Press **Ctrl+C** to stop the server.

## Export a static website

```sh
cargo run -- export --dir /path/to/your/notes --output ./site
```

Omit `--dir` to export the current working directory. The default output is `./markhaus-site`. The output directory must not already exist.

The command writes `index.html`, `style.css`, and an HTML page for each discovered Markdown document, then exits without starting a server. Open the exported `index.html` directly or upload the output directory to a static host. Navigation and styles use relative URLs, so the site also works under a URL subdirectory.

Markdown links to local `.md` and `.markdown` files are rewritten to their exported pages, preserving query strings and fragments. External links are retained. As with the live reader, local images and other assets are not copied. Links in raw HTML are not rewritten. Exported content is a snapshot; run export again to a new output directory to include edits.

## Documents

| Feature       | Behavior                                                                           |
|---------------|------------------------------------------------------------------------------------|
| File types    | `.md` and `.markdown`, case insensitive                                            |
| Discovery     | Recursive, sorted by relative path                                                 |
| Exclusions    | Hidden files/folders, `target`, `node_modules`, symbolic links                     |
| Markdown      | CommonMark plus tables, task lists, footnotes, and strikethrough                   |
| Links         | Relative links to other Markdown files work from the reader                        |
| Images        | External HTTP(S) images and data images are supported; local assets are not served |
| Embedded HTML | Sanitized to remove scripts and unsafe attributes                                  |
| Layout        | Responsive homepage and reader, with print styles                                  |

The server only serves Markdown documents under the selected directory. Paths outside that directory and symbolic links return 404. Filenames must be valid UTF-8, and Markdown contents must be UTF-8 text. By default, the server listens on localhost.

## Development

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Built with [Axum](https://docs.rs/axum/), [pulldown-cmark](https://docs.rs/pulldown-cmark/), and [Ammonia](https://docs.rs/ammonia/). The UI uses local CSS, system fonts, and no JavaScript.
