libwebp, the one C library inside a built-in component, is compiled with wasi-sdk by a `bun nv` tool
into a wasm static library committed under `extensions/image/`, beside the hash of the source it was
built from. CI rebuilds the library from that source on every change to it and fails when the bytes
differ from the committed file.

That keeps `cargo build` free of a C toolchain for every contributor — only a change to libwebp needs
wasi-sdk — while the committed bytes stay a function of reviewable source rather than something a
reviewer has to trust. The library runs only inside the sandbox, which is the condition under which
`rule:packaging/a-c-dependency-answers-two-questions` admits it.

**Not on disk.** There is no `extensions/` tree and no such tool.
