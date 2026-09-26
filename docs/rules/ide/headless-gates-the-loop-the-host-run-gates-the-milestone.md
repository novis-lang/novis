Both tiers of the extension's tests run on every acceptance sweep, and they answer different questions
at costs two orders of magnitude apart.

**Headless, every iteration.** Plain Node, no editor, no display, no network: the grammar snapshot tests, a
contributions test asserting `package.json` declares what the extension claims and depends only on the
allowlist, and a protocol round-trip that spawns the real `nvs lsp` binary and drives it with
`vscode-languageclient`. It runs once rather than once per leg — a `command` check is not a program
fixture, so it has no calling convention for the WSL leg to exercise. CI runs it on all three platforms,
since a `.vsix` is cross-platform and a path bug is not.

**The extension host, wherever a developer works.** `@vscode/test-electron` runs Mocha inside the real
extension host — the only thing that can prove activation on `.nvs`, the Tasks, the `LanguageStatusItem`,
the AST panel and the semantic-token legend. It is on the loop's acceptance list as `npm run test:host`,
so it runs on the desktop the loop runs on, opening a window for about a minute; in CI it runs on Linux
under `xvfb-run -a`.

**Isolation is what makes that safe, and it is not negotiable.** The run uses a downloaded pinned build
rather than the editor the developer installed, `--user-data-dir` and `--extensions-dir` under
`.vscode-test/`, `--disable-extensions`, and a copy of the fixture folder as its workspace rather than the
repository. Without those, a second instance attaches to the editor already open and exits with no
results, and a test that writes a setting writes it into that developer's own `settings.json` — the two
reasons this tier was once CI-only. If a pinned build will not start beside the developer's own editor,
pin a different one; never drop a flag to make it start.

**The host verdict is never remembered.** The check carries `memoize = false`, so no stored verdict
answers for it, whatever changed. Every other check is selected by what its runs were observed to read
in the tree, which is what makes a remembered verdict sound; this one also reads a downloaded editor
build and an installed package tree that no footprint records, since only `nvs`, the test binaries and
`bun nv` log what they read (`tools/nv/select/keys.ts`), so a remembered green here would report an
editor run on a machine where no editor started.
