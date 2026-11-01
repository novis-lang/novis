# The WASI interfaces

The `.wit` files in this folder are unchanged copies of the WASI 0.2 interfaces. `nvs-ext` embeds them
in the binary (`crates/nvs-ext/src/pack.rs`), and `nvs ext` writes them into each extension it
creates. `bun nv gen-attribution` reads the three lines below for the binary's notice.

Upstream: WebAssembly/WASI
Version: 0.2.12
License: Apache-2.0 WITH LLVM-exception

The upstream is the WASI repository, <https://github.com/WebAssembly/WASI>, at its `v0.2.12`
interfaces. Each file is byte-identical to the copy the `wasip2` crate's
`1.0.4+wasi-0.2.12` release ships under `wit/deps/`. To update them, copy that release's files over
these, and change `Version:` above.
