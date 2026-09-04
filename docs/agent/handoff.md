# Handoff

## State

**Goal 6, M7. ADR 0139 is whole** — §§ 1, 2 and 4 are all on disk and green. A record something
changed is sent when the program that opened it ends; a read-only request still makes one round
trip.

**The send travels with the record.** `nvs_runtime::Session::write_back` is a `fn(&mut Ctx)` that
`Core\Session::start` and `regenerate` fill in, and `Ctx::end_session`
(`crates/nvs-runtime/src/ctx.rs:1683`) is what calls it — only for a dirty record, and it clears the
flag, so both ends may call it. The two ends are `nvs-host`'s `isolate::finish`, which is where every
HTTP request ends, and `nvs run`'s root task after ADR 0127's exit hooks. A cancelled task sends
nothing, because the send parks and a task being torn down may not. The whole argument, including the
three shapes it is *not*, is `crates/nvs-stdlib/src/session.rs`'s module doc § *Decision: a dirty
record is sent by the program's end*.

**The item's premise was wrong, and the playbook carries it now.** The door at
`crates/nvs-server/src/serve.rs:709` holds the connection's `Ctx`, not the request's, so no
dependency added to `nvs-server` could have reached the record.

**The driver's stage-6 failure is not a regression.** `a_schedule_entry_fires_as_a_root_isolate` is
still genuinely unwritten — there is no `[[schedule]]` surface in `nvs-server` — which `orient.py`
calls this goal's ordinary state. It is the next group.

**The manifest gap the last two sessions hit is closed.** `[context] modules` named no `nvs-runtime`
and no `nvs-cli` pattern; both are in `docs/agent/loop-goal.toml` and `docs/agent/goals/6-server.toml`
now.

## Next group

**ADR 0073's schedule, over one file set:** `crates/nvs-server/src/serve.rs`,
`crates/nvs-config/src/schedule.rs` and `crates/nvs-cli/src/serve.rs`.

- [ ] **Triage the second check before writing either test** (ADR 0073) — a fleet lease is over the
      shared store, which is `nvs-stdlib`'s wire, and `crates/nvs-server/Cargo.toml:12` names
      `hyper`, `nvs-host`, `nvs-config` and `nvs-runtime` and nothing else. So
      `a_fleet_scoped_entry_fires_once_across_the_fleet_under_its_lease` is either misfiled or wants
      the seam ADR 0139 § 4 just used — a `fn` pointer through `nvs-runtime`
      (`crates/nvs-runtime/src/ctx.rs:1163`). Decide which and say so in the check's comment.
- [ ] **Arm the `[[schedule]]` entries beside the accept loop** (ADR 0073) — the boot check that says
      what an entry must answer is `crates/nvs-config/src/schedule.rs:67`; the binary that binds and
      runs is `crates/nvs-cli/src/serve.rs:82`; the loop itself, and why there is no second scheduler
      to hang a timer off, is `crates/nvs-server/src/serve.rs:451`.
- [ ] **`a_schedule_entry_fires_as_a_root_isolate`** (ADR 0073) — `scope` is mandatory and the entry
      is a `spawn script`, so what is asserted is a **root** isolate rather than a child of a
      connection. The fixture that runs one accept loop on a scheduler of its own is
      `crates/nvs-server/src/serve.rs:1865`.

## Backlog

- `nvs run`'s half of the write-back is wired and unasserted — `crates/nvs-cli/src/main.rs:1190`.
- `isolate::finish`'s call is pinned only from `nvs-stdlib`; no `-p nvs-host` case drives it.
- § 3's `db` backend still throws at run time — `crates/nvs-stdlib/src/session.rs`'s module doc.
- Stage 6's other six checks: ADR 0072 §§ 6-7's deferred queue and ADR 0076's four metrics rows.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024 *Revisiting*, narrowed by m7.md.
- Stage 6b: ADR 0083's persistent connections, whose entry rule lands first at `spawn script`.
