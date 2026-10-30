The image, intl, PDF and spreadsheet components are `.nvsx` bytes embedded in every `nvs` binary
and loaded before any `[[extension]]` entry. They are always present: a program may `use
Novis\Image`, `Novis\Intl`, `Novis\Pdf` or `Novis\Spreadsheet` on any host, in any bundle and under
any configuration, and no setting or Cargo feature removes them.

They are still Tier 1. Each call runs in the sandbox with its own instance per request, under the
request's CPU and memory caps, with an empty WASI and no grant
(`rule:packaging/a-guest-has-no-ambient-authority`). The binary is their pin: their digest is computed
at build time and folded into `env_hash` with the compiler's identity. The namespace `Novis\` is
reserved for them, so a loaded extension declaring a class there does not load, and `Core\` stays
reserved for Tier 0 (`rule:core-api/core-means-always-present`).

Nothing is paid until a program calls one. The embedded bytes are compiled on a component's first use
in a process and stored in the artifact cache like any other module
(`rule:packaging/a-wasm-module-cache-reuses-the-artifact-cache`). What it spends is the binary's size:
mostly CLDR data, the codecs and the PDF engine with its fonts, on disk and in the page cache. The intl
component is ICU4X with every CLDR locale compiled in, and its classes are
`rule:core-classes/intl-roster`. The PDF component's classes are `rule:core-classes/pdf-roster`, and
the spreadsheet component's are `rule:core-classes/spreadsheet-roster`.
Should that growth make
the single-file executable impractical for CLI programs, the answer is a smaller locale set, never an
optional component.

- **Memory:** nothing until a program calls a component. Then the compiled module is held once per
  process and shared by every core, and each calling request has its own instance under its own cap.

A build script compiles their Rust crates, under `extensions/` and outside the workspace, for
`wasm32-wasip2`, and packs each with the packer `nvs ext build` uses
(`rule:packaging/nvs-ext-is-the-authoring-tool`). `rust-toolchain.toml` lists the target, so no contributor installs it by hand. Because the
crates are outside the workspace, the ordinary `cargo build` never resolves them for the host target.
A clean build compiles all four for wasm, ICU4X's compiled data among them, and an incremental build reuses
their target directory. The one C library is prebuilt
(`rule:packaging/a-prebuilt-wasm-library-is-rebuilt-in-ci`).

**On disk.** The image crate is `extensions/image/`, excluded from the workspace, and builds for
`wasm32-wasip2` with `simd128` on. Its Novis source half is every `.nvs` file in `extensions/image/nvs/`, which the build packs into `nvs.source`.
`crates/nvs-ext/build.rs` builds it into `target/ext/`, packs it with `extensions/image/manifest.json`
and embeds it as `nvs_ext::builtin::IMAGE` with its digest. `nvs_ext::load::Loader::builtins` reads
its manifest without compiling, and `Builtin::extension` compiles and checks it on first use
(`crates/nvs-ext/tests/builtin.rs`). `nvs` folds the digest into `env_hash` at start. Every compile
types a program against the built-in manifests beside the entries' (`nvs_ext::load::builtin_manifests`),
and a call no `[[extension]]` entry answers reaches the built-in component, compiled on that first
call (`crates/nvs-cli/src/extensions.rs`), so `Novis\Image\Image::info` answers with no `nvs.toml`
(`tests/conformance/novis/image-info-answers-with-no-configuration.nvst`). The loader refuses a
`Novis\` class from every `[[extension]]` entry. The intl crate is `extensions/intl/`, built and
embedded the same way as `nvs_ext::builtin::INTL` with its digest folded in beside the image's. Its
manifest class is `Novis\Intl\Icu`, and its Novis half is the `Novis\Intl` classes
(`crates/nvs-ext/tests/intl.rs`, `tests/conformance/novis/intl-answers-with-no-configuration.nvst`).
The PDF crate is `extensions/pdf/`, built and embedded the same way as `nvs_ext::builtin::PDF`
with its digest folded in beside the other two. Its manifest class is `Novis\Pdf\Engine`, the
interface `wit/pdf.wit`, and its Novis half is `Novis\Pdf\Document` and `Novis\Pdf\Rendered`
(`crates/nvs-ext/tests/pdf.rs`, `tests/conformance/novis/pdf-renders-with-no-configuration.nvst`).
Its exports are `render`, which lays out and draws a document, `merge`, which joins PDFs into one,
`read`, which returns a PDF's facts, and `read-text`, which returns the text of the pages it names.
The spreadsheet crate is `extensions/spreadsheet/`, built and embedded the same way as
`nvs_ext::builtin::SPREADSHEET` with its digest folded in beside the other three. Its manifest class
is `Novis\Spreadsheet\Engine`, the interface `wit/spreadsheet.wit`, and its Novis half is the classes
under `extensions/spreadsheet/nvs/` (`crates/nvs-ext/tests/spreadsheet.rs`). Its exports are the
writer's, which write an xlsx workbook sheet by sheet, the reader's, which read the sheets, rows and
extras of a workbook, and the template's, which fill the placeholders of a workbook and write it.

**Declined:**
- **Embedded, but removable by a setting or a Cargo feature.** Whether `Novis\Image` exists would
  depend on configuration again, the failure `rule:core-api/core-means-always-present` prevents.
- **Installed beside the binary, with an `[[extension]]` entry per deployment.** That is one
  configuration step for features every web application uses, and a program could not know
  whether one exists without asking its deployment.
- **Both finished `.nvsx` files committed.** Tens of megabytes of binary in git on every CLDR or codec
  update.
