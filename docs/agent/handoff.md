# Handoff

## State

**Goal 15 stage 2 has landed whole.** `editors/vscode` exists: the manifest, a committed
`package-lock.json`, `tsconfig.json`, the flat ESLint config, `language-configuration.json`, the
headless runner and a 15-case contributions suite. `npm run lint` and `npm run test:headless` are
both green, and `tools/verify.py` picks the extension step up on its own now that `package.json` is
there.

**The stage 2–7 acceptance check `vscode (headless)` stays red on purpose.** Its `want` needs
`grammar:` and `protocol:` lines and neither suite exists; `contributions:` and `0 failing` are
already printed. It goes green when stage 5 lands the protocol suite, not before.

**There is no `src/` yet, so the manifest declares no `main`.** Stage 5 adds both, and stage 8's
`.vsix` needs `main` before `npm run package` is worth running; `@vscode/vsce` is already a
devDependency. Node 24 and npm 11 are on this machine and the registry is reachable.

## Next group

**Stage 3: the TextMate grammar** — one file set: a new `editors/vscode/syntaxes/nvs.tmLanguage.json`,
the `grammars` contribution beside the language at `editors/vscode/package.json:26`, and a new
`editors/vscode/test/grammar/` the runner already looks for at
`editors/vscode/scripts/headless.mjs:21`. Split by construct family, never by file
(`rule:ide/highlighting-is-two-layers` is the list; do not re-derive or shorten it).

- [ ] **The grammar's skeleton and the dual-mode openers** — `<?nvs`, `<?php`, `<?=` and `?>` with
      inline HTML outside them, contributed at `editors/vscode/package.json:26` and scoped
      `source.nvs`. `rule:ide/highlighting-is-two-layers`; the construct list is
      `docs/decisions/0099.md:298` § 4.
- [ ] **The scope allowlist suite**, first, so every family after it is checked as it is written:
      `editors/vscode/test/grammar/` tokenizes a fixture with `vscode-textmate` and
      `vscode-oniguruma` (two new devDependencies, so the lockfile moves) and asserts every scope it
      emits is a standard name suffixed `.nvs`. `rule:ide/novis-ships-names-not-colours`; the runner
      picks the directory up at `editors/vscode/scripts/headless.mjs:21`.
- [ ] **Keywords, types and qualifiers** — Novis's own keywords, `tainted`/`secret`, `decimal`, type
      annotations in every slot, and `#[...]` attributes told apart from `#` comments, in the same
      `syntaxes/nvs.tmLanguage.json` under the contribution at `editors/vscode/package.json:26`.
      `rule:ide/rejected-syntax-gets-no-colour` is the other half — what must *not* colour as valid —
      and `docs/decisions/0099.md:298` § 4 lists both halves.

## Backlog

- Strings, heredoc and nowdoc (interpolation in the former only) are the rest of stage 3, then stage
  4's `.lspt` case grammar — same directory, same harness, so they are one group with the above.
- Stage 5 (the client, and the `main` the manifest still lacks) and stage 7 (Tasks, the problem
  matcher, the AST panel) share `src/`. Stage 6 (`secret` concealment) needs the client.
- Stage 8's extension-host suite is **not on the acceptance list and must not be added** — it needs a
  display, and this machine's has the developer's own VS Code open on this repository. CI owns it.
- `.vsix` packaging does gate, at stage 8, and wants `main` first.
- **Off path:** `nvs fmt`, rename, extract, workspace symbol search, inlay hints, signature help,
  `documentHighlight`, the Test Explorer, PhpStorm, and publishing to the Marketplace.
- **When this goal's last check goes green M4B is finished**, and the driver takes goal 16 — the body
  rule and `Core\Request::json()`/`jsonAs<T>()` — then goal 17, `Core\Test::request`'s shape.
