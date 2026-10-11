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

The cost is a second test toolchain beside Cargo: npm, TypeScript and a Mocha suite, a CI job for each
tier, and a setup step on a fresh machine that the Rust half does not need. On every acceptance sweep it
also spends one editor launch that a remembered verdict could have skipped, and the pinned build and
throwaway profile under `editors/vscode/.vscode-test/`, once per machine. That minute buys a verdict that
is true; a stale green is worse than the window, because it reports the editor surfaces proved by a suite
that did not run. Nothing changes at run time: no byte of the binary, the server or a request.

**Declined:**
- **The host tier in CI alone.** It proves nothing while CI is not running, and the editor surfaces would
  be reported done on a suite nothing executed.
- **Hashing `.vscode-test/` and `node_modules/` so the host verdict can be remembered.** It widens what
  every check's selection sees, so every check would re-run on every machine to fix one. A check that
  reads something outside the tree opts out with `memoize = false` instead.
- **A pixel tier.** Colour is asserted through the editor's own commands — `_workbench.captureSyntaxTokens`
  for the TextMate layer, `vscode.provideDocumentSemanticTokens` and its legend for the semantic one. A
  screenshot comparison fails on a font, a theme or a scaling factor with no scope wrong.
- **A test surface with a setter on it.** What `activate` returns stays read-only and exposes nothing
  another extension cannot already read. A function on it that changed state would let any installed
  extension unconceal a secret (`rule:ide/redaction-ranges-come-from-the-server`).
- **Skipping the local run when a display is busy.** A tier that runs only when convenient has a green
  that means nothing, and the condition would not show in the sweep's output.
