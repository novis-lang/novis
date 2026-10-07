# Greeting: a Novis extension in Rust

This project builds a WebAssembly component. Novis loads it as the class
`Example\Greeting`.

## What you need

The Rust target `wasm32-wasip2`. Install it with
`rustup target add wasm32-wasip2`.

## Build and test

```sh
cargo build --release --target wasm32-wasip2
nvs ext build
nvs ext test
```

`nvs ext build` writes `greeting.nvsx` and prints its `sha256`.
`nvs ext pin greeting.nvsx` prints the `[[extension]]` entry for your `nvs.toml`.

## The files

- `nvsx.toml` is the manifest: the class name and the Novis signature of each function.
- `wit/greeting.wit` defines the functions. `wit/deps/` defines the `nvs:ext` world.
  Do not change the files in `wit/deps/`.
- `src/lib.rs` is the code.
- `tests/` has the Novis tests that `nvs ext test` runs.
