# Handoff

## State

- Milestone `dossier`, goal `core-db-transaction-and-1-more`. Every member of `Core\Db\Transaction`
  and `Core\Db\Write` carries its feature proofs, and the goal's own six checks are green.
- The floor check `vscode (host) runs isolated from the developer's editor` is green on this
  machine: `npm run --silent test:host -- --nvs D:/mwl/target/debug/nvs.exe` is `12 passing, 0
  failing`. What was red after session 0006 was a `the paste` host case that was never committed
  and is no longer in the tree.
- `python tools/verify.py --doc` is green, and `owners.py --closes` and `playbook.py --closes`
  name nothing for this goal.
- Nothing is blocked.

## Next group

**The extension host tier** — one file set: `editors/vscode/test/host/`, `editors/vscode/src/imports.ts`.

- [ ] A host case pins that a paste into a Novis document arrives with the `use` lines the copied
      names need, which is the only tier that can observe it
      (`rule:ide/a-pasted-type-carries-its-use-line`) — `editors/vscode/src/imports.ts:1` and
      `editors/vscode/test/host/index.ts:1`. The case that found the `editor.pasteAs.preferences`
      defect was never committed, so the fix has no proof in a real editor.
- [ ] The three waits in the colour tier carry a `seen` reading the way the two status waits now do,
      so a red ledger line names the state the editor was in rather than a bare deadline —
      `editors/vscode/test/host/colour.test.ts:143` and `editors/vscode/test/host/colour.test.ts:120`.

## Backlog

- The `the paste` host case and its two fixtures were left untracked and are gone; `rule:ide/a-pasted-type-carries-its-use-line` § *The paste is the editor's* has no host-tier proof.
- `editors/vscode/` has no `# Known gaps` home, so an extension-side gap cannot survive a goal switch.
