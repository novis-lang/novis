# Handoff

## State

**Stage 6's log envelope is closed, both halves.** The record's shape is pinned in
`crates/nvs-stdlib/src/log.rs`; what this session added is the *seam* — `crates/nvs-server/src/trace.rs`'s
`a_requests_log_record_and_its_span_carry_the_same_trace_id` drives `take` over a real arrived
header and reads the line `nvs_runtime::floor::report` wrote back off a buffered diagnostic
channel, so the ids on the record are the door's decision and not a second one minted where the
record was written. `nvs-server` gained a **dev-only** `nvs-render` edge for the record vocabulary
alone; its `Cargo.toml` comment is that decision's home.

**`Core\Db` gap 6 is now a recorded refusal, not an open option.** `stream` will not declare
`{chunk?: uint}` on any driver: `crates/nvs-stdlib/src/db/mod.rs`'s gap 6 owns the argument, which
is that the portal's `Execute` already asks for every row and the walk holds one.

Nothing is blocked on a decision. The next acceptance failure the driver reports outranks the group
below.

## Next group

**ADR 0073 § 3's fleet lease — stage 7's three checks, and the first slice is a *placement*
question rather than code.** One file set: `crates/nvs-server/src/schedule.rs`,
`crates/nvs-config/src/schedule.rs`.

- [ ] **Where the lease's store lives, decided before anything is written.**
      `crates/nvs-server/src/schedule.rs:239` is the skip that keeps a `fleet` entry unarmed, and
      `crates/nvs-config/src/schedule.rs:174` is the boot refusal for one with no shared store — so
      the config half already knows what a store is. The server half cannot reach one:
      `crates/nvs-server/Cargo.toml` names no `nvs-stdlib`, the shared tier is
      `crates/nvs-stdlib/src/cache.rs`, and **no `compare_and_set` or set-if-absent exists anywhere
      under `crates/` yet**. So either the primitive gets a home the server can reach or stage 7's
      `-p nvs-server` check is one of the playbook's misfiled ones; decide that first, in one grep,
      and the other two slices are ordinary after it.
- [ ] **Arm a fleet entry under the lease.** `crates/nvs-server/src/schedule.rs:222` is `arm`'s doc
      and the skip's home, `crates/nvs-server/src/schedule.rs:523` is `fire`, and the module doc's
      § *What is not armed* at `crates/nvs-server/src/schedule.rs:33` is the paragraph that stops
      being true. The goal's standing decision 13 fixes the shape: set-if-absent with an expiry on
      the shared tier, keyed on `name` plus the fire's scheduled instant, renewed while the run is
      in flight — never a new `Core\Cache` member.
- [ ] **The fallback stays the fallback, and says so at boot.**
      `crates/nvs-server/src/schedule.rs:778`'s `a_fleet_scoped_entry_is_not_armed_on_this_host`
      already pins today's note; under standing decision 13 it becomes the check's
      `a_shared_store_with_no_compare_and_set_leaves_the_entry_unarmed_and_says_so`, and the two
      lease cases the check names join it.

## Backlog

- `serverVersion`, gap 5 — `crates/nvs-stdlib/src/db/mod.rs:237` is the inventory; blocked on all
  five drivers holding a version string, which is `nvs-db`'s work and not this module's.
- `streamAs` is owed whole, same gap 5 — `stream` at a written type, as `queryAs` is `query`'s.
- Gap 8's three `queryAs` refusals that are at run time and should be at compile time — stage 8's
  `-p nvs-types` checks name two of them.
- `[context]` gap: the pack printed ADR 0067 §§ 3, 10, 13 but not **§ 4**, which is the section
  every `stream` question is decided under. Add `§4` to `[context] adrs` for this goal.
