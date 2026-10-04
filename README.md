# Markhaus

A small Rust web server that turns the current working directory into a Markdown reading room, with Bauhaus-inspired typography, geometric forms, and primary colors.

## Run

```sh
cargo run
```

Open **http://127.0.0.1:3000**. Click a document to read it as HTML.

The homepage scans the working directory and its subdirectories on **every request**. Each document is read and rendered on **every request**, and responses send `Cache-Control: no-store`. Refresh after adding, editing, or deleting a file; no restart is needed.

To read a different directory, build once and launch the executable from that directory:

```sh
cargo build --release
cd /path/to/your/notes
/path/to/markhaus/target/release/markhaus
```

To change the listening address:

```sh
cargo run -- --addr 127.0.0.1:8080
```

Run `cargo run -- --help` to see the command-line options.

Press **Ctrl+C** to stop the server.

## Documents

| Feature | Behavior |
| --- | --- |
| File types | `.md` and `.markdown`, case insensitive |
| Discovery | Recursive, sorted by relative path |
| Exclusions | Hidden files/folders, `target`, `node_modules`, symbolic links |
| Markdown | CommonMark plus tables, task lists, footnotes, and strikethrough |
| Links | Relative links to other Markdown files work from the reader |
| Images | External HTTP(S) images and data images are supported; local assets are not served |
| Embedded HTML | Sanitized to remove scripts and unsafe attributes |
| Layout | Responsive homepage and reader, with print styles |

The server only serves Markdown documents under its working directory. Paths outside that directory and symbolic links return 404. Filenames must be valid UTF-8, and Markdown contents must be UTF-8 text. By default, the server listens on localhost.

## Development

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Built with [Axum](https://docs.rs/axum/), [pulldown-cmark](https://docs.rs/pulldown-cmark/), and [Ammonia](https://docs.rs/ammonia/). The UI uses local CSS, system fonts, and no JavaScript.
