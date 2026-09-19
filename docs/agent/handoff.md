# Handoff

## State

Goal `lang-statements`: eleven of its thirteen features carry complete feature proofs. `for`,
`switch` and `return` landed this session beside the eight already done, and `python
tools/dossier.py --id '<feature>'` prints `complete.` for each of the eleven. Every example and every
attack in the group runs green (`python tools/dossier.py --run all --group lang:statements`), and
`docs/perf/members.ndjson` carries a current figure for the three new benches.

Two features are left, both anchored in `docs/reference/lang/40-statements.md`. The group's
acceptance check names the later of the two while either is owed, which is the ordinary in-progress
state and not a regression.

The tests column keeps closing by a `// covers:` marker on a case already on disk that nobody had
attributed (`rule:testing/proof-attribution`) — six more this session, all under
`tests/conformance/lang/`. Look for one before writing a new case. Nothing is blocked.

## Next group

**One file set: `docs/reference/lang/40-statements.md` plus the two remaining proof trees under
`lang/statements/`.** One slice is one feature with all its feature proofs
(`rule:testing/feature-proofs`): `about.md`, three examples with blessed `.out` files, one attack,
one bench, and two tests carrying a `covers:` marker.

- [ ] **`lang:statements/expression-statements-blocks-and-declarations`** — owes examples, hostile,
      perf, tests. What a statement is: an expression with a `;`, a block, a typed declaration,
      `var`, and the scope a block does *not* open. `docs/reference/lang/40-statements.md:9`
- [ ] **`lang:statements/statement-forms-that-do-not-parse`** — owes examples, hostile, perf, tests.
      The forms Novis refuses. An example has to run and match its `.out`, so the three here show the
      accepted spelling of each refused form rather than the refusal, and the two `.nvst` cases carry
      the diagnostics with `--EXPECTF-ERROR--`. `docs/reference/lang/40-statements.md:475`

## Backlog

- Both features above are what the group still owes; nothing else in `lang:statements` is open.
- `python tools/dossier.py --record-perf --group lang:statements` after the last bench of the group
  lands — a figure goes stale whenever `docs/reference/lang/40-statements.md` is touched.
