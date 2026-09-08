# Handoff

## State

**Goal 15 — `editors/vscode` — has just started; nothing of it has landed yet.** Goal 14's whole list is
this goal's Stage 1 floor, so the server, its ten requests and its two code actions are all on disk and
answering. `editors/` does not exist.

**This is the first non-Rust source in the repository, and the tooling for it is already there.**
`tools/orient.py` globs `editors/*/src/**/*.ts` for its module map and `tools/verify.py` carries the
extension step and its `npm` plumbing, dormant until the directory exists. If a session finds either
missing, that is stage 2's blocker and belongs here — not a tooling slice invented mid-goal.

**Stage 0 is empty.** Goal 13 landed `|>` and `let`/`is`, so `rule:ide/highlighting-is-two-layers`'s *must not colour as valid*
list is now checkable against real diagnostics; goals 10 and 11 landed `callable<…>` and `///`, which are
colour surface that list predates.

## Next group

**Stage 2: the package, and the identifiers that are public API** — one file set: a new
`editors/vscode/` (`package.json`, `package-lock.json`, `tsconfig.json`, `language-configuration.json`,
its lint config and npm scripts), plus the four `.gitignore` lines.

- [ ] **The package**, `.nvs` only and never `.php`, extension id `nvs-lang.nvs`, with a committed
      `package-lock.json` because `npm ci` needs one.
- [ ] **`language-configuration.json`** — comments, brackets, auto-closing and surrounding pairs,
      indentation and on-enter rules, folding markers, and a **`wordPattern` that includes `$`**.
- [ ] **The contributions test**: `package.json` declares what the extension claims, depends only on the
      allowlist, contributes no colour-customization defaults, and carries the twelve frozen setting and
      command ids listed in the goal prose. Added to later, never renamed.

## Backlog

- Stages 3 and 4 (the two TextMate grammars) are one group — same directory, same headless harness — and
  stage 3 alone is **expected to take more than one session: split it by construct family, never by file.**
  `rule:ide/highlighting-is-two-layers` is the list; do not re-derive or shorten it.
- Stage 5 (the client) and stage 7 (Tasks, the problem matcher, the AST panel) share `src/`. Stage 6
  (`secret` concealment) needs the client, so it follows stage 5.
- Stage 8's extension-host suite is **not on the acceptance list and must not be added** — it needs a
  display, and this machine's display has the developer's own VS Code open on this repository. CI owns it,
  on Linux under `xvfb-run`. `.vsix` packaging is headless and does gate. The goal's *Standing decisions*
  carry the whole rule, including the profile isolation the suite owes wherever it runs.
- **Off path:** `nvs fmt`, rename, extract, workspace symbol search, inlay hints, signature help,
  `documentHighlight`, the Test Explorer, PhpStorm, and publishing to the Marketplace.
- **When this goal's last check goes green M4B is finished**, and the driver takes goal 16 — the body
  rule and `Core\Request::json()`/`jsonAs<T>()` — then goal 17, `Core\Test::request`'s shape, which ends
  the chain. Both were added after this file was written and neither touches this tree. What remains of
  the milestone table after them is M9 onward.
