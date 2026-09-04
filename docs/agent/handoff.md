# Handoff

## State

**Goal 6, M7. ADR 0073's boot is whole and now answers § 6's next fire; nothing arms it yet.**
`nvs_config::schedule::Cron` (`crates/nvs-config/src/schedule.rs:298`) is the parse the boot refusal
is already made from, `pub` for that reason: `Cron::parse` replaces the old `cron_fault`, and
`Cron::next_after` (`:361`) answers the next instant in the entry's own zone. `zone_of` (`:444`) is
`timezone` or the fault `validate` refuses on. `nvs-config` names `jiff` now (ADR 0051 § 4).

**§ 6's two DST rules and POSIX's two-field rule are pinned, in `crates/nvs-config/tests/schedule.rs`**
— gap fires at the transition, repeat fires on the first occurrence only (including when asked from
*inside* the repeated hour), and a day-of-month and a day-of-week that both narrow fire on either.
The `at` helper is where a civil minute becomes an instant, and the playbook bullet says why
`jiff`'s own disambiguation is not enough for the gap half.

**Nothing fires yet.** `nvs-cli` still spawns only the accept loop, so `[[schedule]]` is inert at run
time and `a_schedule_entry_fires_as_a_root_isolate` is genuinely unwritten — the driver's stage-6
failure is that item, not a regression. The fleet half stays triaged out of this goal; the argument
is the comment above that check in `docs/agent/loop-goal.toml`.

**Orientation gap:** the pack printed no ADR 0073 section at all, so §§ 2 and 6 were sliced by hand.
`[context] adrs` in `docs/agent/loop-goal.toml` should carry `0073` §§ 2, 5 and 6 — § 5 is what the
next group is written against.

## Next group

**ADR 0073's ticker, over one file set:** `crates/nvs-server/src/serve.rs`,
`crates/nvs-cli/src/serve.rs`, against the landed `crates/nvs-config/src/schedule.rs:361`.

- [ ] **Arm the entries beside the accept loop** (ADR 0073 § 5) — the loop runs as a task spawned at
      `crates/nvs-cli/src/serve.rs:440`, and the ticker is a second task on that same scheduler, not
      a second scheduler; `crates/nvs-server/src/serve.rs:909`'s `serve_on_this_core` is the shape to
      follow — the crate takes the *how to fire* as a parameter, the way it takes `handler`, because
      compiling the script is `nvs-cli`'s. The clock question is answered:
      `crates/nvs-config/src/schedule.rs:361`'s `next_after` takes the zone `zone_of` returns, and a
      missed interval is skipped rather than replayed (§ 6), so the ticker asks it from *now* and
      never from the last fire. **A `fleet` entry is not armed**: no lease can be taken, and firing it
      on each host's own clock is the exact failure § 3's key exists to prevent.
- [ ] **`a_schedule_entry_fires_as_a_root_isolate`** (ADR 0073 § 5) — a fire is a **root**, not a
      child of a connection, so it spends `[limits]` rather than a connection's budget. The fixture
      that runs one accept loop on a scheduler of its own is `crates/nvs-server/src/serve.rs:1865`
      (`served_by`/`served_under`), and a ticker case wants the same shape with no client thread.
- [ ] **§ 6's `overlap`, once a fire exists to overlap** (ADR 0073 § 6) — `skip` is the default and
      the other two are `queue` (at most one pending, a second dropped and logged) and `kill` (cancel
      at the next safepoint, wait for teardown, then start). The key is already deserialized and
      unread at `crates/nvs-config/src/tree.rs:719`; it is the ticker's state, not the config's.

## Backlog

- `a_fleet_scoped_entry_fires_once_across_the_fleet_under_its_lease` — M8's, per ADR 0073
  *Verification*; it needs an atomic acquire the shared tier's wire does not have.
- `nvs run`'s half of the write-back is wired and unasserted — `crates/nvs-cli/src/main.rs:1190`.
- `isolate::finish`'s call is pinned only from `nvs-stdlib`; no `-p nvs-host` case drives it.
- § 3's `db` backend still throws at run time — `crates/nvs-stdlib/src/session.rs`'s module doc.
- Stage 6's other six checks: ADR 0072 §§ 6-7's deferred queue and ADR 0076's four metrics rows.
- Stage 6b: ADR 0083's persistent connections, whose entry rule lands first at `spawn script`.
