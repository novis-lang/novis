# Greeting: a Novis extension in C

This project builds a WebAssembly component. Novis loads it as the class
`Example\Greeting`.

## What you need

- `wit-bindgen-cli`. It writes the C code for the WIT files in `wit/`.
  Install it with `cargo install wit-bindgen-cli`.
- wasi-sdk, version 25 or newer. It has the `clang` that builds for
  `wasm32-wasip2`. Set `WASI_SDK_PATH` to the folder you installed it in.

## Build and test

```sh
wit-bindgen c wit --world greeting --out-dir gen
"$WASI_SDK_PATH/bin/clang" --target=wasm32-wasip2 -mexec-model=reactor -O2 \
    -Igen -o greeting.wasm greeting.c gen/greeting.c gen/greeting_component_type.o
nvs ext build
nvs ext test
```

`nvs ext build` writes `greeting.nvsx` and prints its `sha256`.
`nvs ext pin greeting.nvsx` prints the `[[extension]]` entry for your `nvs.toml`.

## The files

- `nvsx.toml` is the manifest: the class name and the Novis signature of each function.
- `wit/greeting.wit` defines the functions. `wit/deps/` defines the `nvs:ext` world.
  Do not change the files in `wit/deps/`.
- `greeting.c` is the code.
- `tests/` has the Novis tests that `nvs ext test` runs.
