# Handoff

## State

**Goal `m5-proofs` (M5) is met.** Stage 8 `tsan` was the one check the last acceptance run reported
red, and it is green on this machine: `wsl.exe -- bash tools/tsan.sh` exits 0 on `tsan: clean`, with
`nvs-host`'s 188 tests passing under the sanitizer, and the stage's three workflow greps all answer.
`python tools/verify.py` is 11 of 11 green and `--doc` green.

**That check's ledger line was never its failure.** The run quoted a `fetch_update` deprecation
warning; the failure eleven lines above it was
`blocking::tests::a_blocking_call_goes_to_a_pool_bounded_at_twice_the_core_count`, 24 threads
standing against a bound of 32 on a box also running a release build.
`crates/nvs-host/src/blocking.rs:529` already holds every job on a shared count until the pool is
full, which needs the bound's worth of threads alive at once — that landed after the acceptance run
that quoted it, so nothing in the tree was owed and no code changed this session.

**The deprecation at `crates/nvs-runtime/src/budget.rs:391` stays as it is.** Nightly renamed
`fetch_update` to `try_update` and the tsan leg is this repository's only nightly build; clippy over
the stable tree reports no warnings, so the call is correct where every other leg compiles it.
Chasing the rename would break them to quiet one line of a leg nothing gates on.

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
- `[context] modules` in `docs/agent/loop-goal.toml` is 21 entries; the driver reports that naming
  `crates/nvs-cli/src/runner.rs`, `crates/nvs-host/src/placed.rs` and
  `crates/nvs-runtime/src/ctx/isolate.rs` would pass 18 — left for a person to narrow.
