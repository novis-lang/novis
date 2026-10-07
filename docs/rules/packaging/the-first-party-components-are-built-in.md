The image component and the intl component are `.nvsx` bytes embedded in every `nvs` binary and loaded
before any `[[extension]]` entry. They are always present: a program may `use Novis\Image` or
`Novis\Intl` on any host, in any bundle and under any configuration, and no setting or Cargo feature
removes them.

They are still Tier 1. Each call runs in the sandbox with its own instance per request, under the
request's CPU and memory caps, with an empty WASI and no grant
(`rule:packaging/a-guest-has-no-ambient-authority`). The binary is their pin: their digest is computed
at build time and folded into `env_hash` with the compiler's identity. The namespace `Novis\` is
reserved for them, so a loaded extension declaring a class there does not load, and `Core\` stays
reserved for Tier 0 (`rule:core-api/core-means-always-present`).

Nothing is paid until a program calls one. The embedded bytes are compiled on a component's first use
in a process and stored in the artifact cache like any other module
(`rule:packaging/a-wasm-module-cache-reuses-the-artifact-cache`). What it spends is the binary's size:
mostly CLDR data and the codecs, on disk and in the page cache.

A build script compiles their Rust crates, under `extensions/` and outside the workspace, for
`wasm32-wasip2`, and packs each with the packer `nvs ext build` uses
(`rule:packaging/nvs-ext-is-the-authoring-tool`). `rust-toolchain.toml` lists the target. The one C
library is prebuilt (`rule:packaging/a-prebuilt-wasm-library-is-rebuilt-in-ci`).

**Not on disk.** The image crate is `extensions/image/`, excluded from the workspace, and builds for
`wasm32-wasip2` with `simd128` on. Its one implemented export is `info`; the others return `runtime`.
No build script packs or embeds it, and the intl crate does not exist.
