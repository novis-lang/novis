# Handoff

## State

**Goal 15 stage 4 is whole.** `editors/vscode/syntaxes/nvst.tmLanguage.json` is the second grammar
`rule:ide/case-files-have-their-own-grammar` asks for: a section per `^--NAME--$`, ending at
`(?=^--[A-Z])`, with Novis embedded in `--FILE--`, `--FILE <path>--`, `--SKIPIF--` and `--CLEAN--`,
PHP in `--ORACLE--`, `expect.rs`'s twelve `%` escapes in `--EXPECTF--`/`--EXPECTF-ERROR--`, and every
other section literal with only its delimiter coloured. The manifest contributes it as a second
language, `nvst`, on `.nvst` and `.lspt`. `npm run test:headless` is green at 118 and `npm run lint`
is clean.

**The PHP split the goal flagged is settled, and the grammar's own comment is its home.**
`source.php` is code-only — it does not handle the `<?php` opener — and `text.html.php` is what does,
so `#oracle-code` matches the tag itself and includes `source.php` for what follows.

**The acceptance check `vscode (headless)` is still red on `protocol:` alone** — stage 5's suite,
which cannot go green before then. `grammar:` and `contributions:` both print.

**Still no `src/` and no `main`** — stage 5's, and stage 8's `.vsix` wants `main` before it is worth
running. The pack's `[context] modules` warning about `editors/vscode/src/**` is that absence.

## Next group

**Stage 5: the client** — one file set: `editors/vscode/src/` (new), `editors/vscode/package.json`,
`editors/vscode/test/protocol/` (new) and `editors/vscode/scripts/headless.mjs`. This is the group
that turns the acceptance check green: `protocol:` is the one suite of the three that prints nothing
today.

- [ ] **The client that starts `nvs lsp`** — a new `editors/vscode/src/extension.ts` and `main`
      beside `editors/vscode/package.json:23`'s `activationEvents`, spawning the binary from
      `nvs.path` and falling back to `PATH`, with the `LanguageStatusItem` for health and version.
      `vscode-languageclient` is already the one runtime dependency, at
      `editors/vscode/package.json:159`. `rule:ide/one-server-two-thin-clients` and
      `rule:ide/vscode-is-the-reference-client`; no language logic in TypeScript.
- [ ] **The version handshake that refuses a binary it does not understand** — what `initialize`
      declares is `crates/nvs-lsp/src/capabilities.rs:1`, and the client registers the same legend or
      nothing lines up. `rule:ide/the-extension-refuses-a-binary-it-does-not-understand`.
- [ ] **The protocol round-trip, headless** — a new `editors/vscode/test/protocol/`, picked up by
      `editors/vscode/scripts/headless.mjs:21`'s `ORDER`, driving the real `nvs lsp` from Node over
      stdio. It needs no `vscode` module, which is why it belongs in this tier at all.

## Backlog

- Stage 6: `secret` concealment and `tainted` left alone — `docs/agent/loop-goal.md` § *Stage 6*.
- Stage 7: Tasks, the problem matcher's two regexes against `crates/nvs-diagnostics/src/render.rs`,
  the AST panel — `docs/agent/loop-goal.md` § *Stage 7*.
- Stage 8: the `.vsix` and the extension-host tier, which is CI's and never run here — the goal's
  § *Standing decisions*.
- `orient.py`'s `[context] modules` prints a module's first sentence only, so a table inside one
  (`crates/nvs-test/src/expect.rs`'s `%` escapes) is reachable only as a handoff anchor.
- The extension's README does not yet record the decided-and-rejected pixel tier — the goal's
  § *Standing decisions* says it is that file's job.
