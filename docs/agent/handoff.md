# Handoff

## State

**Goal `m5-proofs` (M5) is met, and stage 2 `the scale` — the one check the last acceptance sweep
reported red — is confirmed green by running it.** `cargo test --release -p nvs-host --lib
a_hundred_thousand_tasks_in_flight_at_once_all_finish_on_one_core` reports `1 passed; 0 failed; 0
ignored`, so the test runs rather than being filtered or skipped: `[profile.release]` sets no
`debug-assertions`, and the `#[cfg_attr(debug_assertions, ignore)]` at
`crates/nvs-host/src/scheduler.rs:2027` therefore only holds it out of the debug sweep as its doc
comment says.

**The pack's failing line was stale, not a regression.** It was written by the sweep that ended
`17:06`; `61f28cbe0` renamed the test from `a_hundred_thousand_tasks_are_in_flight_on_one_core` to
the name the check filters on at `18:50:52`, after that sweep and before this run began. No code
changed this session — the tree already held the repair, and `52b8c8852` added the
`session.py --wrap` gate that refuses a DONE whose `cargo-named` checks name tests the tree does not
hold, so this claim is checked twice.

The four gaps this goal owned are `owner: unowned`, each with its reason under
[carried-gaps.md](carried-gaps.md) § *Unowned*. Nothing schedules them: each is a decision the user
takes.

## Next group

**Goal met — the next group is the next goal's**, and `python tools/brief.py` prints which.
If the driver reopens `m5-proofs`, this is what is left, one file set — and every item is a
decision first, which is why all three are unowned rather than queued:

- [ ] **A path entry crosses, or a placement that cannot cross refuses instead of falling through
      silently** — `crates/nvs-host/src/placed.rs:34` is the gap and
      `docs/agent/carried-gaps.md:719` the reason.
      `rule:concurrency/on-worker-runs-the-child-on-another-core` states the limit as it stands, and
      `docs/decisions/0184.md` § *Diagnostics* is what rejects the silence.
- [ ] **A serving core registers an inbox, so `nvs serve` places on a sibling rather than starting a
      thread** — `crates/nvs-host/src/worker.rs:99`, reason at `docs/agent/carried-gaps.md:728`,
      and `docs/decisions/0184.md` § *Revisiting* is the fallback that was taken.
- [ ] **The test suite runs its isolates in parallel** — `crates/nvs-cli/src/runner.rs:85`, reason at
      `docs/agent/carried-gaps.md:736`; `docs/decisions/0079.md:158` promises both halves and only
      the isolation half is built.

## Backlog

- `crates/nvs-runtime/src/identity.rs:598`'s `use std::hash::Hasher as _` is unused on nightly and
  needed on stable, so the tsan leg prints one warning nothing can remove from both legs at once.
- `crates/nvs-runtime/src/budget.rs:391`'s `fetch_update` is deprecated on nightly only, which is
  the tsan leg alone; clippy over the stable tree is clean, so the call stays as it is.
- `[context] modules` in `docs/agent/loop-goal.toml` is 21 entries; the driver reports that naming
  `crates/nvs-cli/src/runner.rs`, `crates/nvs-host/src/placed.rs` and
  `crates/nvs-runtime/src/ctx/isolate.rs` would pass 18 — left for a person to narrow.
