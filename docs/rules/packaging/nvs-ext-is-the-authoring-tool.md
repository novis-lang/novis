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

**Not on disk.** There is no `nvs ext` subcommand.
