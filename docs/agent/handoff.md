# Handoff

## State

- Milestone `dossier`, goal `core-db-transaction-and-1-more`. Every member of `Core\Db\Transaction`
  and `Core\Db\Write` carries its feature proofs, and the goal's own six checks are green.
- Every perf figure in the tree is current again. Adding headings to three `docs/reference/lang/`
  chapters staled the 39 figures measured off them, which is `rule:testing/member-perf-ledger`
  working; `python tools/dossier.py --record-perf` re-measured exactly those and `--perf-report`
  rewrote `docs/perf/members.md`.
- The extension host tier is `13 passing, 0 failing` on this machine:
  `npm run --silent test:host -- --nvs D:/mwl/target/debug/nvs.exe` from `editors/vscode`.
- `python tools/verify.py --doc`, `owners.py --closes` and `playbook.py --closes` are green for this
  goal.
- Nothing is blocked.

## Next group

**The extension host tier** — one file set: `editors/vscode/test/host/colour.test.ts`,
`editors/vscode/test/host/index.ts`.

- [ ] The three waits in the colour tier carry a `seen` reading the way the status and paste waits
      now do, so a timeout in the driver's sweep names what the editor was showing instead of a bare
      deadline (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`) —
      `editors/vscode/test/host/colour.test.ts:51`.
- [ ] `ID`, `fixture`, `open` and `until` stand in three copies across this tier now; one
      `editors/vscode/test/host/editor.ts` holds them, and Mocha does not load it because the in-host
      index takes `*.test.js` alone
      (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`) —
      `editors/vscode/test/host/index.ts:29` and `editors/vscode/test/host/colour.test.ts:36`.

## Backlog

- The copy half of paste-with-imports has no proof in any editor — `editors/vscode/src/imports.ts`
  § *Known gaps* 1, owner `M10`.
- 731 of 1154 features still owe a proof, which is the feature-proofs program's own debt —
  `python tools/dossier.py --owed`.
- A `.nvs` comment in a host fixture is not held to `AGENTS.md` § *Text an end user reads*, and the
  tier's own voice is what the existing fixtures use — worth a sentence in
  `editors/vscode/test/host/index.ts` if it is ever asked again.
