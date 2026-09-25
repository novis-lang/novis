Every tool that builds, checks, renders or drives this repository is a subcommand of one program,
`bun nv <command>`, written in TypeScript and run by Bun at the version `package.json` pins. Its one
runtime dependency is `smol-toml`, which reads the TOML files the tree still has; `Bun.TOML` is never
used, so one parser reads all of them. The dev dependencies are `typescript` and `@types/bun`, and any
other dependency is a new decision. One part is Rust: `tools/nv-scan`, which the build keys read Rust
source with, because only a parser reads Rust exactly. It is a crate outside the workspace, which
`bun nv` builds and runs, and its dependencies are `syn`, `proc-macro2`, `quote` and `sha2` at the
versions the workspace already locks. `bun nv audit python` fails on a tracked Python file it does not
name as allowed, and on a document that tells a reader to run a Python tool.

This is the repository's tooling, not the language's: nothing here reaches the `nvs` binary, a Novis
program or a request.
