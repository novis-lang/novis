# Handoff

## State

Goal `core-io-file-and-1-more` is complete: all thirteen `Core\IO\File` and `Core\IO\Metadata`
members carry every proof of `rule:testing/feature-proofs`, and both of the goal's dossier checks
pass. `verify.py --doc`, `owners.py --closes` and `playbook.py --closes` are green for it.

The floor check `nvs-runtime (the live list and the sweep)` went red after session 0005 with
`dismantling_through_a_foreign_context_panics_in_debug` "did not run". The fix (commit ac3899b4f:
`verify.py`'s `TEST_LINE_RE` matches `- should panic`, and `loop.py`'s `verify_green` refuses a
record with fewer test lines than its result counts) was already on disk. The turn that checked
session 0005 had loaded `loop.py` before that commit. With the current code, `Goal.crate_tests("nvs-runtime")`
refuses the stale 418-of-420 record, runs the binary and finds the name (probe:
`.agent-tmp/probe_runtime_check.py`). The stored record in `.agent-tmp/verify-test-green.json`
stays short until `verify.py` re-runs that binary; the refusal makes that harmless.

## Next group

**Goal complete** — the driver picks the next goal in the chain; its first session overwrites
this file.

## Backlog

- The db-matrix floor checks went red once on a cold boot (mysql/mariadb TCP legs: `nvs_jobs`
  missing, then a `CREATE TABLE` failing) and passed 4/4 on a rerun. If it recurs, the bring-up
  wait in `tools/db-matrix.py` is the place to read.
