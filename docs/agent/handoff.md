# Handoff

## State

**M8 goal 5, stage 10.** `Core\Taint::assertTrusted` is landed whole (ADR 0024 § 3); the
`secret` axis deliberately does not reach it, and `crates/nvs-stdlib/src/taint.rs`'s module
doc owns why there is no `assertTrustedBytes`.

**The array copy-on-write write path is audited and clean — do not re-audit it.** Every
`extern "C"` mutator in `crates/nvs-runtime/src/array.rs` writes its handle back through
`into_raw()` *before* it decides on a fault, so no early return between `make_unique` and an
entry point can drop a clone. The two that can fault are pinned by counting tests over
`live_bytes` at `crates/nvs-runtime/src/array.rs:2659` and `:2699`: the append refuses
*before* it separates (reachable from source as `$b = $a; $a[] = 1;` at `i64::MAX`, and
`emit_array_append` loads the yielded pointer only on the continuation edge, so the ordering
is load-bearing), and the spread hands back the separation it had already made.

**The 292-byte leak is still open, and its valgrind stack has been re-read.** `make_unique`
under `nvs_array_set` is the allocation site of *every* non-empty array literal since the
empty-array singleton landed — see the new playbook bullet — so the stack says "a literal was
never released" and says nothing about the write path. It was seen only during a five-wide
sweep of `examples/transaction.nvs`, whose sessions died on an *error* path a green run never
takes, which is where to look next.

**The driver's failing acceptance check is stale driver state, not a regression, and no
session in this run can clear it.** Thirteen valgrind fixtures are reported red with `exit 1`,
which the `tools/loop.py` on disk cannot emit — session 0004 fixed the sweep and the running
driver still holds the import from before it. The playbook bullet is the evidence. Do not
re-fix it.

**Orientation gap, carried:** `[context]` still has no field that can name a `docs/spec/` file.

## Next group

**One file set: `crates/nvs-stdlib/src/db.rs`, with a new fixture under `tests/db/` and
`tools/leak-check.sh` as the harness.** Do **not** run copies of `examples/transaction.nvs`
concurrently — they race on `create table accounts` and die on `pg_type_typname_nsp_index`,
which is the harness and not the bug.

- [ ] **Reproduce the sweep's failure in one process, on the throwing statement path.** A
      fixture that connects and runs the same `create table` twice, catching the second's
      `Db\DbError`, run under `wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh`. The reader
      the throwing path abandons is at `crates/nvs-stdlib/src/db.rs:4201`, and the two
      members are `crates/nvs-stdlib/src/db.rs:6366` and `crates/nvs-stdlib/src/db.rs:6449`.
      Helper arguments are borrowed, not owned (`crates/nvs-runtime/src/abi.rs:523`), so a
      leaked argument array would be the *caller's* cleanup, not the member's.
- [ ] **If that is green, take the uncaught throw instead** — the sweep's sessions exited on
      an uncaught `DbError`, which tears down through `crates/nvs-runtime/src/abi.rs:462`
      rather than through a `catch`, and a `raise_with_slots` whose descriptor is too narrow
      drops a slot's value silently (the playbook's `Ctx::pending_slot` bullet).
- [ ] **Pin whichever one reproduces with a counting test over `live_bytes`**, beside the two
      added this session at `crates/nvs-runtime/src/array.rs:2659`.

## Backlog

- `emit_array_spread`'s error edge does not load `out_p`, so a *shared* spread destination
  would leak the copy and double-release the original; unreachable today because a literal
  under construction is solely owned unless it is the empty singleton, which has no next
  integer key to occupy. Stated at `crates/nvs-codegen/src/emit.rs:3016`, held from the
  runtime side by `a_refused_spread_hands_back_the_separation_it_had_already_made`.
- `[context]` in `docs/agent/loop-goal.toml` needs a field that can name a `docs/spec/` file.
- Stage 10's remaining corpus items — `docs/agent/loop-goal.toml`.
