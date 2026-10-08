`nvs ext` is the one tool an extension author needs beyond their language's compiler:

| Command | What it does |
|---|---|
| `nvs ext new --lang rust\|c <dir>` | writes a project that builds a component against `nvs:ext@1.0.0` |
| `nvs ext build` | takes the author's built wasm module or component, componentizes it, writes the manifest from the project's `nvsx.toml`, embeds the Novis source, and checks the result loads |
| `nvs ext inspect <file>` | prints the manifest and the I/O it requests; `--source` prints the source |
| `nvs ext test` | runs the project's `.nvs` tests with the built file loaded and no `nvs.toml` |
| `nvs ext verify <file>` | runs every load check on a file |
| `nvs ext pin <file>` | prints the `[[extension]]` entry, with its `sha256`, ready to paste |

Rust and C are the two templates because they are the two languages the first-party components prove.
`nvs ext build` takes a module from any toolchain, so Zig, Go and every other language with a wasm
target work by hand. The packer behind `nvs ext build` is the same code the binary's own build uses for
the built-in components (`rule:packaging/the-first-party-components-are-built-in`).

**Not on disk.** `nvs ext` and its six subcommands parse. `nvs ext build` reads `nvsx.toml`, packs
the project, runs every load check and writes the `.nvsx` beside `nvsx.toml`, printing its path and
pin; it refuses a WASI preview 1 module naming `wasm32-wasip2`. `nvs ext new` writes the Rust or C
template into a new or empty folder, with the world's WIT copied from the binary under `wit/deps/`,
and `rust-toolchain.toml` lists `wasm32-wasip2`. The Rust template builds, loads, passes `nvs ext test`
and is called from a program: the `wasi:cli/terminal-*` imports its standard library makes link as
refusing stubs (`crates/nvs-ext/src/wasi.rs`), and `bun nv ext-templates`, which CI's `ext-templates`
job runs on three platforms, does every step; the C template is not checked here. `nvs ext inspect`
prints the manifest, the I/O it requests and the source paths, and with `--source` the source, every
string from the file with its control characters escaped; `nvs ext verify` runs boot's loader on a file
under its own pin and instantiates nothing; `nvs ext pin` prints the entry with its absolute `path` and
no `grants`, and only for a file that loads. `nvs ext test` runs the `#[Test]` methods under the
project's `tests` folder with the built file loaded, reads the files `--config` names and never
`./nvs.toml` or the data folder's, grants nothing without one, and refuses a file older than any input of its build or one
a `--config` entry pins to another digest (`crates/nvs-cli/tests/ext_command.rs`). The packer is: `nvs_ext::pack` takes a component, or
a core module with the author's WIT, plus a manifest and source files, refuses a source path that leaves
the project, and writes a `.nvsx` that loads, the same bytes each time (`crates/nvs-ext/tests/pack.rs`).
