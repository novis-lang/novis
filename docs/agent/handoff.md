# Handoff

## State

**Goal `resource-ceilings`: stage 3's first check is green, and its second is one test short.**
Both copies of the goal TOML (`docs/agent/loop-goal.toml` and `docs/agent/goals/40-resource-ceilings.toml`,
byte-identical again) now name the tests the tree holds. Two of that check's four names were claims
landed under other names; the other two named real work and are written —
`crates/nvs-host/src/watchdog.rs:1168` pins that a core parked past its ceiling in *wall* time is
not flagged, and `crates/nvs-host/src/cpuclock.rs:@no_ceiling_note` is what a platform with no
per-thread clock says at boot, printed by `crates/nvs-cli/src/serve.rs:372` as the fleet starts.

**The stage's second check is two renames and one unwritten test.** `nvs_safepoint`'s back edge and
`bounded_loop`'s in-member poll are pinned under the tree's names, re-pointed here; nothing asserts
that a limit handler overrunning its widened ceiling is stopped rather than entered a second time,
which is the next group. The stage's third check is the `.nvst` case, which is on disk.

**One store stays unbracketed on purpose**, still the compiled-pattern cache —
`crates/nvs-stdlib/src/regex.rs`'s gap 4 is the finding, waiting on M6's arena.

## Next group

**Stage 3: the limit handler's zero-retry, the last name in the stage's second check** — one file
set, `crates/nvs-runtime/src/ctx/hooks.rs` with `crates/nvs-runtime/src/ctx/limits.rs` beside it and
`crates/nvs-host/tests/limits.rs` as the fixture to copy. `rule:errors/on-limit` is what all of it
is a claim about.

- [ ] **Read how a test builds a registered limit handler before writing one** —
      `crates/nvs-host/tests/limits.rs:304` is the existing fixture and it is in `nvs-host`, while
      the check is `args = ["test", "-p", "nvs-runtime"]`. If no `nvs-runtime` test can build a
      closure value, the check's `args` is the half that is wrong and the name moves to the crate
      that owns the fixture — the playbook's *a check can name a test in a crate that cannot host
      it*.
- [ ] **Write `the_limit_handler_runs_once_and_is_not_re_entered_when_it_overruns`** —
      `crates/nvs-runtime/src/ctx/hooks.rs:543` is the slice: `run_limit_handler` widens
      `cpu_limit` by `fatal_reserve_time`, lowers `SafepointFlags::CPU_LIMIT` for the length of the
      call and raises only what it lowered on the way out. The claim is that a handler which burns
      past the widened ceiling is stopped there rather than re-entered, which is
      `crates/nvs-runtime/src/ctx/limits.rs:404`'s zero-retry rule.
- [ ] **Then re-point or keep the third name, in both TOML copies** —
      `docs/agent/loop-goal.toml:8043` and `docs/agent/goals/40-resource-ceilings.toml:8043` hold
      the same list, and the comment above it names what is missing; the copies are byte-identical
      and must stay so.

## Backlog

- `nvs run` says nothing where the platform has no clock — `crates/nvs-cli/src/main.rs:1993` builds
  `RunningRequest` from `ThreadClock::current()` and is silent when it is `None`.
- This goal's own record is still unopened; next free is `docs/decisions/0175.md`, re-checked
  before it is claimed — goal `resource-ceilings` § *Standing decisions*.
- The compiled-pattern cache stays unbracketed until M6's arena —
  `crates/nvs-stdlib/src/regex.rs` § *Known gaps*, gap 4.
- Stage 3's `[context]` printed no `crates/nvs-cli/src/serve.rs`; it was needed for the boot site
  and the driver's `modules` sweep now carries it.
