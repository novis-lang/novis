Every tool that builds, checks, renders or drives this repository is a subcommand of one program,
`bun nv <command>`, written in TypeScript and run by Bun at the version `package.json` pins. Its one
runtime dependency is `smol-toml`, which reads the TOML files the tree still has; `Bun.TOML` is never
used, so one parser reads all of them. The dev dependencies are `typescript` and `@types/bun`, and any
other dependency is a new decision. `bun nv audit python` fails on a tracked Python file it does not
name as allowed, and on a document that tells a reader to run a Python tool.

This is the repository's tooling, not the language's: nothing here reaches the `nvs` binary, a Novis
program or a request.
