# Handoff

## State

Goal `lang:iteration` is **met**: `python tools/dossier.py --verify --group lang:iteration` reports
`nothing owed` and `0 failed` twice over its six features. `what-foreach-walks`,
`the-two-interfaces` and `what-does-not-exist` landed their feature proofs this session, joining the
three that were already complete.

`what-does-not-exist` is excused its perf figure in `tools/data/dossier-policy.toml`, by the
precedent of `lang:expressions/refused-in-expression-position`: every name in it is a compile error,
so there is no program to iterate. Its attack carries `// hostile: expect-refusal`, and the compiler
answers it with 32 errors and no crash.

The two benches are deliberately a pair: the array subject measures 21.97 units with 0 calls and 0
allocations, the hand-written cursor 151.55 units with 18 calls and 1 allocation per round. That gap
is the cost of `rule:iteration/foreach-subjects`'s interface forms, and it is now on the ledger.

`python tools/owners.py --closes lang-iteration` and `python tools/playbook.py --closes
lang-iteration` both report nothing owned, and `python tools/verify.py --doc` is green. Nothing is
blocked.

## Next group

**Stage: feature proofs for the next generated goal** — the chain's next entry is goal `lang-errors`
(`docs/agent/goals/dossier/86-lang-errors.md`), and a goal switch overwrites this file with its own
handoff. These items stand only if the driver stays on `lang:iteration`.

- [ ] **Re-read the group ledger before writing anything** — `python tools/dossier.py --group
      lang:iteration` prints all six features and what each holds.
      `rule:testing/feature-proofs`. `docs/reference/lang/60-iteration.md:9`
- [ ] **A chapter edit re-stales every perf figure in it** — the ledger keys a `lang` feature's
      figure to its reference file, so editing `docs/reference/lang/60-iteration.md` marks all six
      stale at once and each needs `--record-perf` again. `rule:testing/member-perf-ledger`.
      `docs/reference/lang/60-iteration.md:422`

## Backlog

- Goal `limit-handler-reach` still owes the `onLimit` half of the memory ceiling — `rule:errors/on-limit`.
- `lang:iteration/the-two-interfaces` sits at 151.55 units, past `[report.ceiling] constant = 100`
  in `tools/data/dossier-policy.toml`; advisory only, and honest for an interface-dispatched loop.
