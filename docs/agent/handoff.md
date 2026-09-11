# Handoff

## State

**Goal `resource-ceilings`: every stage is on disk, including stage 7.** The record is
`docs/decisions/0174.md`, which creates the three rules the goal's § *Standing decisions* names —
the refusal and its degenerate return, the accounting boundary, and the expansion rule — and both
of stage 7's checks exit 0 (`python tools/rules.py --check`, `python tools/decisions.py --gate`).
The previous handoff opened stage 7 as unlanded work; it had landed in an earlier run, and the
group it named would have written a duplicate record at the next free number.

**The floor check the driver last reported red was a clock flake, and it is fixed.**
`examples/pool.nvs` measured its two waits with `Core\Time::now` while the `acquire` bound is a
monotonic deadline in the pool, so a realtime step made a 500ms wait read as 464ms. Both readings
are `Core\Time::monotonic` now; the fixture passes natively and three runs out of three under WSL.

**One store stays unbracketed on purpose**, still the compiled-pattern cache —
`crates/nvs-stdlib/src/regex.rs`'s gap 4 is the finding, waiting on M6's arena.

The pack printed nothing about `examples/` or `crates/nvs-stdlib/src/db/`, which is correct for
this goal's own stages and is why `[context] modules` is left alone: the check that failed belongs
to a carried-forward floor, not to `resource-ceilings`.

## Next group

**The goal's acceptance, and the one clock exposure beside the fixture just fixed** — one file
set, `examples/` and `tests/conformance/core/`, with nothing under `crates/` to touch.

- [ ] **Read the driver's acceptance line in `.loop/log.md` before opening anything else** — every
      stage of this goal is on disk and its rulebook gate is green at
      `docs/agent/loop-goal.toml:8158`, so the last red was the floor flake this session closed.
      `rule:programs/memory-priority` is the rule the goal works inside. If the run comes back
      clean, the status line is `DONE` and no further slice is owed.
- [ ] **Widen the wall-clock half of the monotonic sweep, which has the exposure
      `examples/pool.nvs` just lost** —
      `tests/conformance/core/time-monotonic-is-only-ever-compared-with-itself.nvst:111` prints
      `$wallMs >= 30` across a `Core\Time::sleep(30ms)`, so a backward realtime step inside that
      sleep answers `no` and fails the `--EXPECT--` block below it.
      `rule:http-server/every-deadline-is-monotonic` is why the two clocks are not interchangeable;
      the claim to keep is that *both* clocks notice a wait, so the assertion widens rather than
      moving to the monotonic reading.

## Backlog

- The compiled-pattern cache takes no accounting bracket until M6's arena — `crates/nvs-stdlib/src/regex.rs` § *Known gaps*.
- `[context] modules` sits at 18 entries and the ledger has twice named `crates/nvs-cli/src/serve.rs` as the one it would add — `docs/agent/loop-goal.toml`.
