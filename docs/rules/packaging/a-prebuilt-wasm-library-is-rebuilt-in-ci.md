libwebp, the one C library inside a built-in component, is compiled with wasi-sdk by a `bun nv` tool
into a wasm static library committed under `extensions/image/`, beside the hash of the source it was
built from. CI rebuilds the library from that source on every change to it and fails when the bytes
differ from the committed file.

That keeps `cargo build` free of a C toolchain for every contributor — only a change to libwebp needs
wasi-sdk — while the committed bytes stay a function of reviewable source rather than something a
reviewer has to trust. The library runs only inside the sandbox, which is the condition under which
`rule:packaging/a-c-dependency-answers-two-questions` admits it.

**Partly on disk.** `bun nv webp-lib` builds libwebp into `extensions/image/libwebp/libwebp.a`, and
`SOURCE.json` beside it pins the source tarball and the wasi-sdk release by sha256 and records the
library's own. The build runs on Linux, and under WSL on Windows, and refuses a tarball whose digest
differs from its pin. `--check` holds the committed bytes to their digest, and CI's `webp-lib` job
runs `--verify`, which rebuilds and compares (`tools/nv/test/webp-lib.test.ts`). The image crate
does not link the library yet.
