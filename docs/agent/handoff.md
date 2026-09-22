# Handoff

## State

- Milestone `dossier`, goal `core-db-transaction-and-1-more`. Every member of `Core\Db\Transaction`
  and `Core\Db\Write` carries its feature proofs; the goal's own six checks, `verify.py --doc`,
  `owners.py --closes` and `playbook.py --closes` are all green for it.
- The chain is 178 goals. `tools-server` is new at 175 — the eight features of the `nvs serve`
  reference chapter — and `plain-comments`, `foreach-var` and `ci-green` moved to 176, 177 and 178.
  Everything at or below the live goal was left alone, which is the emitter's own rule.
- `python tools/verify.py` is 14 of 14 green, and the extension host tier is `13 passing, 0 failing`
  (`npm run --silent test:host -- --nvs D:/mwl/target/debug/nvs.exe` from `editors/vscode`).
- `ID`, `DEADLINE`, `fixture`, `open`/`shown` and `until` are one module,
  `editors/vscode/test/host/editor.ts`. Mocha never loads it because the in-host index globs
  `*.test.js` alone, and `index.ts` now says so.
- Nothing is blocked.

## Next group

**The remaining bare waits in the surfaces tier** — one file set:
`editors/vscode/test/host/surfaces.test.ts`, `editors/vscode/test/host/editor.ts`.

- [ ] The two diagnostics waits carry a `seen` reading — how many diagnostics the document holds and
      which source published them — so a timeout says whether the server never answered or never
      cleared (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`) —
      `editors/vscode/test/host/surfaces.test.ts:106`.
- [ ] The problemMatcher wait and the AST panel wait carry one too: what the Problems panel held,
      and what `reading.ast.getChildren` answered instead of a tree
      (`rule:ide/the-ast-panel-shells-out-to-the-cli`) —
      `editors/vscode/test/host/surfaces.test.ts:124` and
      `editors/vscode/test/host/surfaces.test.ts:150`.
- [ ] `seen` is optional on the shared `until`, and every call site in the tier now passes one; make
      it required so a wait added later cannot be written without a reading —
      `editors/vscode/test/host/editor.ts:50`.

## Backlog

- The copy half of paste-with-imports has no proof in any editor — `editors/vscode/src/imports.ts`
  § *Known gaps* 1, owner `M10`.
- 692 of 1154 features still owe a proof, which is the feature-proofs program's own debt —
  `python tools/dossier.py --owed`.
- A `.nvs` comment in a host fixture is not held to `AGENTS.md` § *Text an end user reads*, and the
  tier's own voice is what the existing fixtures use — `editors/vscode/test/host/index.ts` is where
  a sentence would go if it is ever asked.
