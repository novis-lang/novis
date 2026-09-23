# Handoff

## State

Goal `core-io-file-and-1-more` is complete: all thirteen `Core\IO\File` and `Core\IO\Metadata`
members carry every proof of `rule:testing/feature-proofs`, and both of the goal's dossier checks
pass. The floor sweep it was reached on went red on three things, and each is now closed:

- `dismantling_through_a_foreign_context_panics_in_debug` "did not run". It ran and passed, but
  `verify.py`'s `TEST_LINE_RE` did not match libtest's `- should panic` line. The pattern now
  matches it. `loop.py`'s `verify_green` also refuses a record that has fewer test lines than its
  result line counts, so the old records (nvs-runtime, nvs-db, nvs-ir, nvs-repo, nvs-codegen
  `calls`) are re-run rather than reused.
- `Core\IO::lines` and `stdin` perf was stale after the `Metadata` edit to `io.rs`. All 26
  `Core\IO` figures were re-measured into `docs/perf/members.ndjson`. `dossier.py --verify --group
  'Core\IO'` owes nothing.
- The four db-matrix checks: a by-hand `db-matrix.py --all` failed the mysql and mariadb TCP legs
  (`nvs_jobs` missing, then a `CREATE TABLE` failing). A rerun of both drivers passed 4/4. That is
  flaky, not a regression; see the backlog.

## Next group

**Goal complete** — the driver picks the next goal in the chain; its first session overwrites
this file.

## Backlog

- The mysql/mariadb TCP legs of `tools/db-matrix.py` failed once on the shared `nvs_jobs` table and
  passed on a rerun. Find out whether the four matrix checks in the floor run at the same time
  against one server, or whether two queue tests race inside one process
  (`crates/nvs-stdlib/tests/queue.rs`). Owner: `tools/db-matrix.py`.
