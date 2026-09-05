# Handoff

## State

**Goal 6, M7 — ADR 0083 § 1 is landed end to end for a path entry.** The connection's door
offers the slot, `Core\Socket::upgrade` fills it, and the connection starts what the request
left: the member resolves a path through `nvs_runtime::script::resolve` (which is ADR 0118
§ 2's own `script.spawn` door, so this module declares no capability row of its own), crosses
`args:` by ADR 0023 § 2's graph copy, and leaves the pair in `nvs_runtime::UpgradeSlot`.
`crates/nvs-stdlib/src/socket.rs`'s module doc is the one home of why the member prepares and
the connection starts. **Still no `101` and no framing** — an upgrade prepared today opens a
root isolate with no peer attached to it.

**The method entry form throws, and the reason is recorded rather than open.** ADR 0006 binds
`args:` by name; a `Class::method(...)` reference reaches a `Core` call as the closure
`lower_callable_ref` built, and a closure carries arity and parameter tags but **no parameter
names** — the sibling `spawn script` construct only works because `nvs_ir::lower`'s
`spawn_method_entry` writes the names in as a constant, which an ordinary argument has no room
for. Binding by position instead was refused. `crates/nvs-stdlib/src/socket.rs`'s § *The
method form waits on a name* owns the decision and names the fix: a
`CLOSURE_PARAM_NAMES_SLOT` written at the literal, which serves every later `CoreTy::Entry`
row too.

**§ 5's decision is made and is in the ADR.** `Core\Sse::upgrade` does **not** travel through
`UpgradeSlot`: an SSE connection takes no socket, its response is an ordinary `200
text/event-stream` whose body the isolate writes while the connection future still runs, so it
is a **second cell** on the carrier offered to every request the server runs. ADR 0083 § 5's
body is the home of that. Nothing of it is built — `Core\Sse` is still unregistered — so
`sse_is_a_connection_isolate_with_no_receive` is still the goal's one failing acceptance check,
and it is now an implementation item rather than a design one.

## Next group

**`Core\Sse`, from the registry row outwards.** Items 1 and 2 share
`crates/nvs-stdlib/src/socket.rs` and `crates/nvs-runtime/src/ctx/inbound.rs`; item 2 also
touches `crates/nvs-server/src/serve.rs`, which item 3 reads.

- [ ] **The second cell** — ADR 0083 § 5's `Core\Sse` cell on the carrier, beside
      `crates/nvs-runtime/src/ctx/inbound.rs:667`'s `upgrade_slot` and modelled on
      `crates/nvs-runtime/src/ctx/inbound.rs:746`'s `UpgradeSlot`, whose `fill`
      (`crates/nvs-runtime/src/ctx/inbound.rs:768`) refuses a second. Offered to **every**
      request rather than to an upgradable one, which is the whole difference.
- [ ] **`Core\Sse::upgrade`'s five edits** — the row, the card, the body, the `address()` arm
      and three `.nvst` cases, beside `Core\Socket`'s at
      `crates/nvs-stdlib/src/socket.rs:155` and the body below
      `crates/nvs-stdlib/src/socket.rs:184`. `CoreTy::Entry` again, `send` and **no**
      `receive`; `crates/nvs-stdlib/src/registry.rs:2253`'s `entry_parameter` picks the row up
      with no second roster to edit.
- [ ] **The door offers it, and the check splits** — `crates/nvs-server/src/serve.rs:697` is
      where the upgrade offer is made and where the SSE cell is offered unconditionally;
      `crates/nvs-server/src/serve.rs:820` is where one is taken. The acceptance check
      `sse_is_a_connection_isolate_with_no_receive` (`docs/agent/loop-goal.toml:3746`) is a
      conjunction — "no receive" is a `-p nvs-stdlib` registry fact and "is a connection
      isolate" is `-p nvs-server` — so it splits the way the playbook's conjunction bullet
      describes.

## Backlog

- `CLOSURE_PARAM_NAMES_SLOT`, which unblocks `Core\Socket::upgrade`'s method entry —
  `crates/nvs-stdlib/src/socket.rs`'s § *The method form waits on a name*.
- The framing slice: `101`, `tungstenite` over `NvsStream`, and the socket into the isolate —
  `crates/nvs-server/src/serve.rs:542`'s doc.
- `Core\Topic` (ADR 0083 § 4), unregistered — `crates/nvs-stdlib/src/socket.rs`'s known gaps.
- `nvs_host::Scheduler`'s `finished` retention, the half no session took —
  `crates/nvs-host/src/scheduler.rs:540`.
