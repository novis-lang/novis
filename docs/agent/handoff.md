# Handoff

## State

**Goal 6, M7. ADR 0139 § 1's seven members are all on disk and green.** `start`, `get`, `set`,
`remove`, `clear`, `regenerate` and `destroy` are registered, implemented, carded and addressed in
`crates/nvs-stdlib/src/session.rs`; § 2's four store operations are complete now that `destroy`
has `redis::Connection::del` under it. The record lives on the request as `nvs_runtime::Session` —
id, ADR 0023 bytes, dirty flag — reached only through `Ctx::open_session`/`session`/`session_mut`/
`close_session` (`crates/nvs-runtime/src/ctx.rs:1654`).

**What ADR 0139 still owes is § 4's write-back at the end of the request.** A `set` this build
accepts is visible to the rest of that request and to nothing after it; `regenerate` and `destroy`
reach the store as they land, because § 4 makes those two immediate. The module doc says so out
loud.

**The write-back needs a seam that does not exist yet, and that is the next group's first item.**
`crates/nvs-server/src/serve.rs:709` is where a request ends with its `Ctx` still live, but
`crates/nvs-server/Cargo.toml` names only `hyper`, `nvs-host`, `nvs-config` and `nvs-runtime` —
and `nvs-host` names no `nvs-stdlib` either — so the door cannot reach the store at all. The
installed-trait seam already in the tree is `nvs_runtime::host::install`.

**The driver's stage-5 failure is not a regression.** `a_schedule_entry_fires_as_a_root_isolate` is
still genuinely unwritten — `grep` over `crates/nvs-server/src` finds no schedule surface — so it
is an item still open, which `orient.py` calls this goal's ordinary state.

**One manifest gap, and it cost about four calls.** `[context] modules` names no `nvs-runtime`
entry for `src/array.rs` or `src/value.rs`, so the `NvsArray`/`Value` API every `Core` member that
touches a map is written against had to be re-derived by hand. Add both patterns.

## Next group

**§ 4's write-back, over one file set:** `crates/nvs-runtime/src/host.rs`,
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-stdlib/src/session.rs` and
`crates/nvs-server/src/serve.rs`.

- [ ] **Decide where a dirty record is sent, and record the decision** (ADR 0139 § 4) — the door
      ends the request at `crates/nvs-server/src/serve.rs:709`, with the context still live, and
      cannot call `nvs-stdlib`; `nvs_runtime::host::install` at
      `crates/nvs-runtime/src/host.rs:551` is the trait `nvs-cli` already installs, and a request-
      end method on it is the shape that reaches both sides. Deciding is pre-authorized under the
      goal's § *Standing decisions*; the home for the reasoning is `session.rs`'s module doc, not a
      new ADR.
- [ ] **Send the record the flag earned** (ADR 0139 § 4) — `crates/nvs-stdlib/src/session.rs:559`
      is `write_back`, which sets `dirty` and nothing else; the send is
      `save(open, &id, &record, ttl(ctx))` for a dirty record and *nothing at all* for a clean one,
      which is the whole of "writing only when the record changed". `crates/nvs-runtime/src/ctx.rs:1667`
      is the only route to the flag.
- [ ] **Pin it both ways** (ADR 0139 § 4) — extend the scripted store at
      `crates/nvs-stdlib/src/session.rs:1060`, whose doc already requires a new command to be
      taught to it, and assert that a request that wrote sends exactly one `SET` and one that only
      read sends none. A `.nvst` case cannot reach a store, so this is a `-p nvs-stdlib` `#[test]`
      beside `a_destroyed_record_is_gone_and_forgetting_it_twice_is_not_a_failure`.

## Backlog

- § 3's `db` store: `start` throws naming it — `crates/nvs-stdlib/src/session.rs`'s module doc.
- `a_schedule_entry_fires_as_a_root_isolate`: ADR 0073's `[[schedule]]` has no surface in
  `crates/nvs-server/src` — the driver's standing stage-5 failure.
- `destroy` writes no expiring `Set-Cookie`; ADR 0139 § 1 does not ask for one, and the reasoning
  is in that member's doc if it should.
- ADR 0139 § 5's sweeper and the store's own expiry — `docs/adr/0139-…` § 5.
