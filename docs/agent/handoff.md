# Handoff

## State

**Goal 6, M7. ADR 0073's schedule is the open work, and its fleet half has been triaged out of
this goal.** `a_fleet_scoped_entry_fires_once_across_the_fleet_under_its_lease` is gone from stage
6's `-p nvs-server` list: ADR 0073 *Verification* files "two hosts racing the same fleet-scoped
interval produce exactly one run" under **M8**, `nvs-server` cannot name the store, and
`Core\Cache`'s wire (`put`/`get`, ADR 0059 § 2) holds no compare-and-set for a lease to be taken
with anywhere in the tree. The whole argument is the comment above that check in
`docs/agent/loop-goal.toml`, mirrored in `docs/agent/goals/6-server.toml`; the backlog carries the
name.

**§ 3's boot question is now a real one.** `nvs_config::schedule::configures_a_shared_store` reads
`[cache.shared] url` instead of answering `false` unconditionally, so a `fleet` entry beside a
configured store boots and one without it still refuses.
`a_fleet_scope_with_no_shared_store_refuses_the_boot` holds both sides plus the empty-`url` case.

**Nothing fires yet.** No ticker exists: `nvs_config::schedule::validate` checks an entry and
`nvs-cli` spawns only the accept loop, so `[[schedule]]` is inert at run time and
`a_schedule_entry_fires_as_a_root_isolate` is genuinely unwritten — the driver's stage-6 failure is
that item, not a regression.

## Next group

**ADR 0073's ticker, over one file set:** `crates/nvs-config/src/schedule.rs`,
`crates/nvs-server/src/serve.rs` and `crates/nvs-cli/src/serve.rs`.

- [ ] **The next-fire computation, beside the parse that already validates it** (ADR 0073 §§ 2, 6) —
      `crates/nvs-config/src/schedule.rs:99` refuses a bad `cron` through `cron_fault`, and
      `crates/nvs-config/src/schedule.rs:46`'s `FIELDS` is the dialect; the same parse should answer
      the next instant rather than a second one growing in `nvs-server`. `jiff` 0.2 is the
      workspace's clock (`jiff = "0.2"` in the root `Cargo.toml`) and `nvs-config` does not name it yet — adding it is
      pre-authorized (ADR 0051 § 4), and its `Zoned` disambiguation *is* § 6's two DST rules: a
      civil time in a spring-forward gap resolves to the instant after the gap, one in a fall-back
      repeat to the first occurrence. An unknown `timezone` should refuse the boot for the module
      doc's own reason — a schedule fails silently, so the question is asked once at boot.
- [ ] **Arm the entries beside the accept loop** (ADR 0073 § 5) — the loop runs as a task spawned at
      `crates/nvs-cli/src/serve.rs:440`, and the ticker is a second task on that same scheduler, not
      a second scheduler; `crates/nvs-server/src/serve.rs:909`'s `serve_on_this_core` is the shape
      to follow — the crate takes the *how to fire* as a parameter, the way it takes `handler`,
      because compiling the script is `nvs-cli`'s. **A `fleet` entry is not armed**: no lease can be
      taken, and firing it on each host's own clock is the exact failure § 3's key exists to
      prevent, while not firing is inside § 3's stated at-most-once.
- [ ] **`a_schedule_entry_fires_as_a_root_isolate`** (ADR 0073 § 5) — a fire is a **root**, not a
      child of a connection, so it spends `[limits]` rather than a connection's budget. The fixture
      that runs one accept loop on a scheduler of its own is `crates/nvs-server/src/serve.rs:1865`
      (`served_by`/`served_under`), and a ticker case wants the same shape with no client thread.

## Backlog

- `a_fleet_scoped_entry_fires_once_across_the_fleet_under_its_lease` — M8's, per ADR 0073
  *Verification*; it needs an atomic acquire the shared tier's wire does not have.
- `nvs run`'s half of the write-back is wired and unasserted — `crates/nvs-cli/src/main.rs:1190`.
- `isolate::finish`'s call is pinned only from `nvs-stdlib`; no `-p nvs-host` case drives it.
- § 3's `db` backend still throws at run time — `crates/nvs-stdlib/src/session.rs`'s module doc.
- Stage 6's other six checks: ADR 0072 §§ 6-7's deferred queue and ADR 0076's four metrics rows.
- Stage 6b: ADR 0083's persistent connections, whose entry rule lands first at `spawn script`.
