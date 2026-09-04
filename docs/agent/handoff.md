# Handoff

## State

**Goal 6, M7. ADR 0073's ticker fires.** `nvs_server::schedule`
(`crates/nvs-server/src/schedule.rs`) is a second task on the accept loop's own scheduler: it sleeps
until the soonest fire `nvs_config::schedule::Cron::next_after` names, spawns each due entry as a
task of its own, and runs the caller's isolate there. It takes *how to fire* as a parameter — the
`Fires` trait, three questions — exactly as the crate takes `handler`, because turning a `script`
path into a program needs the front end. `nvs serve`'s half is `Scheduled`
(`crates/nvs-cli/src/serve.rs:562`), resolving per fire through the same `nvs_runtime::script`
resolver a request uses, so ADR 0017's unit swap reaches a nightly job at its next fire.

**§ 6's clock rule is the implementation, not a comment**: every next fire is asked from *now*, so a
missed interval is skipped rather than replayed, and the two DST answers stay in `next_after` where
the boot already put them. `a_missed_interval_is_skipped_rather_than_replayed` pins the skip at
`Armed::rearm`, and `a_schedule_entry_fires_as_a_root_isolate` pins § 5's four readings of one fire
— it runs at its minute, on a task that is not the tick's, with **no request on the isolate**, and
its result logged rather than delivered. The clock is a parameter of `tick_on_this_core` for that
test's sake; `nvs serve` passes `Zoned::now`.

**A `fleet` entry is still not armed**, and that is the refusal ADR 0073 § 3 asks for rather than a
gap: no lease can be taken anywhere in this tree (`Core\Cache` has no compare-and-set), so `arm`
skips it and names it at boot. The acceptance check's own comment owns that triage.

## Next group

**ADR 0073 § 6's `overlap`, over one file set:** `crates/nvs-server/src/schedule.rs`, with the
directive at `crates/nvs-config/src/tree.rs:720` and the boot's reader beside it.

- [ ] **`overlap = "skip"`, the default** (ADR 0073 § 6) — a fire is spawned unconditionally today,
      so an entry whose run outlives its interval overlaps itself. The tally the ticker already
      keeps is process-wide; what `skip` needs is a per-entry one, read where the due list is built
      at `crates/nvs-server/src/schedule.rs:250` and given back by the same `Ran` guard at
      `crates/nvs-server/src/schedule.rs:326`. `Armed` (`crates/nvs-server/src/schedule.rs:70`)
      is where the mode belongs, read once by `arm` at `crates/nvs-server/src/schedule.rs:163`.
- [ ] **`queue` and `kill`, decided and recorded** (ADR 0073 § 6) — `queue` is one deferred fire
      held per entry, which the same counter answers; `kill` needs a running isolate to be torn
      down, and whether this tree can do that is `crates/nvs-host/src/scheduler.rs`'s cancellation
      question. If it cannot, the safe half is a boot refusal of `overlap = "kill"` beside the
      others in `crates/nvs-config/src/schedule.rs:96` — a decided-and-recorded call, not an ADR.
- [ ] **Per-entry `limits` and `grants`** (ADR 0073 § 5) — narrowing only, and nothing in this tree
      narrows a budget *per isolate* yet. Carry them on `Armed`
      (`crates/nvs-server/src/schedule.rs:70`) and spend them in `Scheduled::isolate`
      (`crates/nvs-cli/src/serve.rs:564`), which is the only side holding a `Ctx` to set them on.

## Backlog

- `an_after_response_tree_outlives_its_connection` and `the_deferred_queue_is_bounded_by_max_concurrent`
  — ADR 0072 §§ 6-7, and nothing in `crates/nvs-server/src/serve.rs` spells `afterResponse` yet.
- The three ADR 0076 names in the same acceptance check (trace id, sampling, no probe on the
  measured path) — `docs/agent/loop-goal.toml`'s stage 6 block is the list.
- A `fleet` entry's lease — ADR 0073 *Verification*'s M8 bullet, waiting on a shared store that can
  compare-and-set.
- ADR 0073's boot could compile a scheduled `script` the way § 2 compiles a mounted entry; today a
  broken one is found at its first fire (`docs/adr/0097` § 2's argument, applied to schedules).
