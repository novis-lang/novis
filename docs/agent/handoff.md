# Handoff

## State

**Goal 6, M7. ADR 0073 is whole bar `fleet`.** `nvs_server::schedule` fires a `[[schedule]]` entry as
a root isolate beside the accept loop (§ 5), and § 6's `overlap` now answers an entry that comes due
while its own last run is still going. The mode is read once by `arm` onto `Armed`
(`crates/nvs-server/src/schedule.rs:70`), and the count it is asked of is that entry's own rather than
the process-wide tally the tick keeps for its teardown. `skip` drops the fire and names it, `queue`
holds exactly one and starts it the moment the run before it ends, `kill` cancels the task that run
was spawned as and waits out its teardown. Every mode rearms from the clock, so a dropped fire never
leaves the entry due — that would make the next wait zero and spin for as long as the run lasts.

**Two mechanisms carry the two non-default modes**, and the module doc § *An entry never overlaps
itself* owns both: a held fire waits on a run *ending*, so the tick's wait is
`nvs_host::timer::wait_until` while anything is held; and `kill` keeps each fire's `TaskId` for
`nvs_host::cancel_task`, then parks on the entry's count, which the `Ran` guard gives back however
the run ended.

**The boot refuses a fourth word** — `nvs_config::schedule::validate` answers `E0611` for an
`overlap` that is none of the three, beside its `scope` refusal. The roster's reader keeps a fallback
note for the hole that closes, the way its other unreachable notes do.

**A `fleet` entry is still not armed**, unchanged: ADR 0073 § 3's refusal, because no lease can be
taken anywhere in this tree.

## Next group

**ADR 0072 §§ 6-7's deferred work, over one file set:** `crates/nvs-server/src/serve.rs` and
`crates/nvs-runtime/src/deferred.rs`, with the directive at `crates/nvs-config/src/tree.rs:669`. This
is what the failing acceptance check names — `nvs-runtime`'s half is landed and nothing in the server
calls it. `[context] adrs` now carries 0072 §§ 6-7, so the pack states both rules.

- [ ] **`run_deferred` runs after the response, on a tree the connection no longer holds**
      (ADR 0072 § 6) — `nvs_runtime::deferred::run_deferred`
      (`crates/nvs-runtime/src/deferred.rs:108`) has no caller in this crate. The completion a
      connection answers from is `crates/nvs-server/src/serve.rs:434`, the response is built at
      `crates/nvs-server/src/serve.rs:770`, and the in-flight guard that ends a connection is
      `crates/nvs-server/src/serve.rs:1148`. Test: `an_after_response_tree_outlives_its_connection`,
      and it is hostable where it is filed — `crates/nvs-server/Cargo.toml` names `nvs-runtime`.
- [ ] **`[deferred] max_concurrent` is counted, and a registration past it throws at the call site**
      (ADR 0072 § 7) — the gap is named at `crates/nvs-runtime/src/deferred.rs:59`, the directive at
      `crates/nvs-config/src/tree.rs:669`, and the member that must throw rather than queue is
      `crates/nvs-stdlib/src/task.rs:561`. The count belongs where the registration lands, not at the
      member. Test: `the_deferred_queue_is_bounded_by_max_concurrent`.

## Backlog

- Per-entry `limits` and `grants` (ADR 0073 § 5) — carry them on `Armed`
  (`crates/nvs-server/src/schedule.rs:70`), spend them in `Scheduled::isolate`
  (`crates/nvs-cli/src/serve.rs:564`); nothing in this tree narrows a budget per isolate yet.
- The three ADR 0076 names in the same acceptance check (trace id, sampling, no probe on the measured
  path) — `docs/agent/loop-goal.toml`'s stage 6 block is the list.
- A `fleet` entry's lease — ADR 0073 *Verification*'s M8 bullet, waiting on a shared store that can
  compare-and-set.
- ADR 0073's boot could compile a scheduled `script` the way § 2 compiles a mounted entry; today a
  broken one is found at its first fire (ADR 0097 § 2's argument, applied to schedules).
- This session's item was entirely ADR 0073 § 6 and the pack printed no section of that ADR — sliced
  by hand. The group is closed, so nothing was added for it; the pattern is what `[context] adrs`
  costs when a group opens without its section.
