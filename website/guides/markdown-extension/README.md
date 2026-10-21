# Markdown: a Novis extension in Rust

This project builds a WebAssembly component. Novis loads it as the class
`Example\Markdown\Parser`. The component uses the Rust library `pulldown-cmark`
to convert Markdown to HTML.

The extension also contains one Novis class, `Example\Markdown\Document`. It
calls the component and makes the HTML safe to show on a page.

The guide *Write an extension in Rust* on the Novis website explains every file
and every step.

## What you need

The Rust target `wasm32-wasip2`. Install it with
`rustup target add wasm32-wasip2`.

## Build and test

```sh
cargo build --release --target wasm32-wasip2
nvs ext build
nvs ext test
```

`nvs ext build` writes `parser.nvsx` and prints its `sha256`.

## Try the demo page

```sh
cd demo
nvs ext pin ../parser.nvsx > nvs.toml
nvs serve index.nvs
```

Open `http://127.0.0.1:8000/`. Write Markdown in the box, and the page shows it as HTML.

## The files

- `nvsx.toml` is the manifest: the class name and the Novis signature of each function.
- `wit/markdown.wit` defines the functions. `wit/deps/` defines the `nvs:ext` world.
  Do not change the files in `wit/deps/`.
- `src/lib.rs` is the Rust code.
- `nvs/Document.nvs` is the Novis class that is packed with the component.
- `tests/` has the Novis tests that `nvs ext test` runs.
- `demo/index.nvs` is a web page that uses the extension.
