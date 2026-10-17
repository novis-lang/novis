The editor client sits at the repository root, beside `crates/` and outside the Rust workspace:

```
editors/
  vscode/      TextMate grammar, language-configuration.json, LSP client extension
```

It is not a Cargo crate. The VS Code extension is TypeScript and Node tooling, so the workspace
`Cargo.toml` does not govern it, and it brings a build toolchain (`npm`/`vsce`) that is a genuinely new
kind of CI job next to everything `cargo` builds. `nv verify` runs the extension's headless suites last, for that reason,
and treats the directory being absent as a real state rather than an error.

A directory is created when the work that fills it starts, never scaffolded empty ahead of it — the rule
every crate already follows. The VS Code package and the server crate
that back it are the only ones there are (`rule:ide/one-crate-and-one-extension-grow-in-place`).
