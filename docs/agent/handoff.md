# Handoff

## State

**Goal 13 is met: every check in `loop-goal.toml` passes.** `python tools/dossier.py --only
lang:expressions/the-pipeline-operator --gate` says nothing is owed and `python tools/reference.py
--check` says `docs/novis.md` is current at 286 of 286 examples. The gate's last debt was not a
missing test — it was `perf: stale`, because stage 2 gave the operator a reference heading and
`docs/reference/lang/30-expressions.md` is what its figure is fingerprinted against
(`rule:testing/member-perf-ledger`). `--record-perf` re-measured it at 21.370 units.

**The depth gap the previous session found is closed, and it was never the pipeline's alone.** A
loop that left-nests a tree was charged nothing by `enter_recursive`, so the guard bounded the
parser's stack and not the tree handed to the first pass that walks it recursively. `parse_pipe`,
`parse_left_assoc` and `parse_instanceof` now each charge one level per link and hold it to the end
of the chain, as `parse_postfix` already did. A 4000-term `$n + 1 + 1 …` was accepted before this
and is refused with `E0108` now; the full gate is green either side of the change, so nothing
legitimate in the corpus sat above the limit.

**Another agent holds this tree.** ADR 0150, `docs/rules/types/type-test.md` and a
`30-type-test` goal are uncommitted work that is not this session's; only the four files named in
this session's commits were staged.

## Next group

**Both proofs of the new bound, from the Novis side, one file set:** `tests/conformance/syntax/`,
`tests/hostile/lang/`, `crates/nvs-syntax/src/parser/expr.rs`.

- [ ] **A `.nvst` case pins the refusal from the Novis side**, since the bound is currently proved
      only from Rust and `rule:testing/four-proofs` wants both — an `--EXPECTF-ERROR--` case over a
      flat chain past the limit, under `tests/conformance/syntax/`.
      `crates/nvs-syntax/src/parser/expr.rs:70` is `parse_left_assoc`, whose `links` is what the
      case exercises, and `crates/nvs-syntax/src/parser/mod.rs:441` is the `E0108` it must
      reproduce, indentation and all.
- [ ] **A hostile case for the flat spelling**, which is the attack the previous session's
      pipeline attack could not reach because it went after the parser's one hole slot instead —
      `rule:testing/hostile-case-contract` is the contract, so it passes by nothing coming apart
      and a compile diagnostic is never the pass. `crates/nvs-syntax/src/parser/expr.rs:559` is
      `parse_pipe`'s `stages`, and the case belongs beside the existing ones in
      `tests/hostile/lang/`.

## Backlog

- Parse-time nesting has no rule of its own; `enter_recursive`'s doc is its only home — `rule:errors/stack-depth` is the run-time bound, not this one.
- `docs/perf/members.ndjson` holds 4 records for 1 machine, so any figure re-measured elsewhere will not compare — `docs/perf/members.md`.
- The `[context]` manifest printed everything this session needed; no field was missing.
