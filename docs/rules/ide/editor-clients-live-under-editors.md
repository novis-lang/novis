The editor clients sit at the repository root, beside `crates/` and outside the Rust workspace:

```
editors/
  vscode/      TextMate grammar, language-configuration.json, LSP client extension
  phpstorm/    file-type registration, LSP-bridge plugin (Kotlin/Gradle)
```

Neither is a Cargo crate — the VS Code extension is TypeScript and Node tooling, the PhpStorm plugin is
Kotlin, Gradle and the IntelliJ Platform SDK — so neither is governed by the workspace `Cargo.toml`, and
each brings a build toolchain (`npm`/`vsce`, Gradle) that is a genuinely new kind of CI job next to
everything `cargo` builds. `nv verify` runs the extension's headless suites last, for that reason,
and treats the directory being absent as a real state rather than an error.

A directory is created when its milestone starts, never scaffolded empty ahead of it — the rule every
crate already follows. The VS Code package and the server crate that back it are then the only ones
there will be (`rule:ide/one-crate-and-one-extension-grow-in-place`).
