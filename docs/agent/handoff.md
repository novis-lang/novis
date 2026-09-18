# Handoff

## State

Goal `types-enum-2-2` — types:enum (2/2) — is met: all 17 enums carry a description, three examples
and an attributed test, and the goal's own dossier check has been green since session 0005.

**The `the CLI's own start work stays under 6ms [1 floor]` failure was the machine, not the tree.**
The sweep's own log has the proof: at 23:19 the check read a 4.6 ms start floor and 5.4 ms of work,
and at 00:55 — four seconds after a 34 s `cargo test --release` — it read a **77.5 ms** floor, a
277.9 ms total and 200.3 ms of "work". Load on this box dilates a process multiplicatively, so
subtracting the floor does not leave Novis's own work, and the spread says nothing about it: that
run's median sat 4% over its minimum, as tight as any idle one, because every rep was equally slow.
`tools/bench.py`'s `warm_start` now reads the *level* of the floor instead and abstains above
`QUIET_FLOOR` times the budget, the way `benches/abi-probe`'s fan-out guard abstains on its control.

Two red runs of the old guard were reproduced by hand this session and both abstain now. The same
leg also takes `WARM_START_REPS` reps instead of the suite's five, because idle the figure moved a
full millisecond between consecutive runs — more than the 6 ms budget has to give — and twenty-five
holds it inside three tenths with the floor steady to a tenth.

No Rust changed; one Python file did. `python tools/verify.py` is **13 of 13 green** (4879 tests,
2088 conformance, 279 differential, clippy clean) — on the third attempt. The first two died in the
test leg on two different load-sensitive tests, each of which the harness re-ran alone and passed;
the playbook now carries that as its own trap.

## Next group

**Goal `types-enum-2-2` is met, so the driver's goal switch installs goal `types-exception`'s own
generated handoff over this one.** Its next three exceptions, in the order `TREE` declares them, so
the next session does not re-derive the group — one file set: `crates/nvs-hir/src/errors.rs`,
`docs/examples/types/<name>/` and `tests/hostile/types/<name>/`, with `rule:testing/four-proofs`
naming what each owes and `crates/nvs-codegen/tests/arithmetic.rs:40-57` the spelling a `catch`
clause and a `->message` read take.

- [ ] **`Core\Cli\NotInteractive`** — page, three examples, an attack and a `covers:` marker.
      `crates/nvs-hir/src/errors.rs:106`
- [ ] **`Core\Db\DbError`** — the same four; a `RuntimeError` subclass, so a `catch (RuntimeError …)`.
      `crates/nvs-hir/src/errors.rs:107`
- [ ] **`Core\Db\RolledBack`** — the same four, and the neighbour in that file.
      `crates/nvs-hir/src/errors.rs:108`

## Backlog

- `nvs --version` has no guard of its own, so a regression in the start floor silences the
  warm-start abstain rather than tripping it — said in `warm_start`'s own note in `tools/bench.py`.
- This box stays saturated for tens of seconds after a `cargo test --release`; `COST_SETTLE` in
  `tools/loop.py` gave 30 s and the retry was still dilated. Worth raising if a guard flaps again.
- Other cost-class guards read the same dilated machine and have no control of their own;
  `benches/abi-probe/tests/perf_guards.rs` is where that would be answered next.
- An `nvs.exe` from 2026-09-16 is still resident on this box (2 s of CPU in three days, so it is
  idle rather than the load): a test or example left a process behind and nothing sweeps them.
