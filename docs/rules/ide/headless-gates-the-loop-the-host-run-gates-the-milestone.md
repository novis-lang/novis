Two tiers, because they answer different questions and cost two orders of magnitude apart.

**Headless, every iteration.** Plain Node, no editor, no display, no network: the grammar snapshot tests, a
contributions test asserting `package.json` declares what the extension claims and depends only on the
allowlist, and a protocol round-trip that spawns the real `nvs lsp` binary and drives it with
`vscode-languageclient`. It is what the loop's acceptance test gates on, and it runs once rather than once
per leg — a `command` check is not a program fixture, so it has no calling convention for the WSL leg to
exercise. CI runs it on all three platforms, since a `.vsix` is cross-platform and a path bug is not.

**The extension host, in CI only.** `@vscode/test-electron` runs Mocha inside the real extension host —
the only thing that can prove activation on `.nvs`, the Tasks, the `LanguageStatusItem`, the AST panel and
the semantic-token legend. It needs a display, and the only display on a developer's machine is one a
person is using, so it runs on Linux under `xvfb-run` and is not on the loop's acceptance list at all.
Wherever it runs it isolates its profile — `--user-data-dir` and `--extensions-dir` to a throwaway
directory, a fixture folder rather than the repository — or a test that writes a setting writes it into
the developer's own `settings.json`.
