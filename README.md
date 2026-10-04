# Markhaus

Turn a folder of Markdown files into a reading room. Markhaus gives your notes and documents a Bauhaus-inspired layout, with bold typography, geometric shapes, and primary colors.

Read your documents locally, or export them as a static website to share.

## Get started

You’ll need Rust and Cargo installed. From the Markhaus project folder, install the command:

```sh
cargo install --path . --locked
```

Then open a terminal in the folder containing your Markdown files and run:

```sh
markhaus
```

Open **http://127.0.0.1:5779** in your browser. The homepage lists the Markdown files in that folder and its subfolders. Click a document to read it.

Edit your files in your usual editor, then refresh the browser to see your changes. New and deleted files appear when you refresh the homepage. There’s no need to restart Markhaus.

Press **Ctrl+C** in the terminal to stop it.

## Choose a folder or port

To read documents from another folder:

```sh
markhaus --dir /path/to/your/notes
```

The folder must already exist. You can use an absolute path or a path relative to your current folder. Put paths containing spaces in quotes.

To use a different port:

```sh
markhaus --addr 127.0.0.1:8080
```

Open the address you chose in your browser. By default, Markhaus is available on your own computer at port **5779**.

## Export a website

Save your collection as a website that can be opened without running Markhaus:

```sh
markhaus export --dir /path/to/your/notes --output ./site
```

Leave out `--dir` to use your current folder. Leave out `--output` to save to `./markhaus-site`. The output folder must not already exist; choose a new folder for each export.

Open `index.html` inside the output folder to read the exported collection. To publish it online, upload the contents of that folder to any static website host. Keep the files and subfolders together so navigation and styling continue to work. The website also works when hosted under a URL subfolder.

An export is a snapshot of your documents. After editing your Markdown, export again to a new folder and replace the published files with the new output.

Links written in Markdown to other local `.md` or `.markdown` documents point to their exported pages. External links stay unchanged. Links written directly in HTML are kept as written.

## Supported documents

Markhaus finds `.md` and `.markdown` files, including uppercase extensions, and lists them by their path. Hidden files and folders, `target`, `node_modules`, and symbolic links are skipped. Documents and filenames must use UTF-8 text.

Tables, task lists, footnotes, and strikethrough are supported. The layout adapts to smaller screens and includes print styles for printing or saving a page as a PDF from your browser.

Images hosted online can appear in your documents. Local images and other attachments aren’t served or included in exports, so use hosted image URLs for pictures you want readers to see. Embedded HTML is supported, with scripts and unsafe attributes removed.

## Command help

```sh
markhaus --help
markhaus export --help
```

To run from the project folder without installing, use `cargo run --` in place of `markhaus`, for example:

```sh
cargo run -- --dir /path/to/your/notes
```
