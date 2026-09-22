# Handoff

## State

- Milestone `dossier`, goal `core-db-transaction-and-1-more`. The goal's own six checks and the
  `Core\Db\Transaction` and `Core\Db\Write` proofs were already green; what was red was the **floor**,
  where commit e548fc412 added four reference headings and the derived roster grew three features that
  owed everything.
- All three are closed. `lang:programs/doc-comments` and
  `lang:attributes/core-program-implementingwith-i-t-member-every-implementor-with-one-attribute` were
  written from nothing; `lang:expressions/closures-fn` was a **rename** of `lang:expressions/closures`,
  whose five proofs were on disk under the old id.
- Every one of the 13 groups the floor carries was gated this session, and those three were the only
  holes. `python tools/dossier.py --verify` is green for `lang:programs`, `lang:expressions` and
  `lang:attributes`.
- `python tools/verify.py` is 14 of 14 green (2319 conformance, 5176 unit, 309 reference examples).
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

- `dossier.py --comments` counts a `///` run as a comment above a step, so a proof *for* doc comments
  misses the two-line bound by construction — as the landed `lang:programs/comments` files already do.
  Goal `plain-comments` owns it, and it has to be settled there before the bounds become a gate.
- A hostile case misses the 25-word bound by design: an attack's whole point is the line nobody writes
  by hand. Same goal, same decision.
- `docs/perf/members.ndjson` keeps the old `lang:expressions/closures` rows as history; the ledger is
  append-only and the new id was measured beside them, so there is nothing to clean up.
