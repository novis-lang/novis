# Handoff

## State

**Goal 6, M7 — ADR 0083 § 1's carrier is on disk and nothing fills it yet.**
`nvs_runtime::Upgrade` is the prepared connection isolate — a `nvs_runtime::script::Program`
and the already-crossed `args`, exactly `nvs_host::Isolate::new`'s first two arguments
(`crates/nvs-runtime/src/ctx/inbound.rs:693`) — and `nvs_runtime::UpgradeSlot` is the shared
cell it is left in (`crates/nvs-runtime/src/ctx/inbound.rs:746`), whose `fill` refuses a
second upgrade by handing it back rather than overwriting. `nvs_server::serve_connection`
offers one half to a request `hyper` framed an upgrade for and to no other, keeping the other
half beside the `OnUpgrade` it took off the extensions (`crates/nvs-server/src/serve.rs:666`);
the offer reaches the request's carrier through `nvs_host::Isolate::offering_upgrade`
(`crates/nvs-host/src/isolate.rs:211`), because the door never holds an `Inbound` — the
handler it asks for a reply is what builds one. **The design's home is unchanged**:
`crates/nvs-stdlib/src/socket.rs`'s module doc, § *Decision: this member spawns nothing, and
the connection starts it*.

**The member still throws** (`crates/nvs-stdlib/src/socket.rs:236`), and that module's doc
owns the gap list. The driver's stage-6b check is now two: § 1's three tests and § 5's one
stay `-p nvs-server`, §§ 2-3's four moved to a `-p nvs-stdlib` check of their own —
`docs/agent/loop-goal.toml:3738` and the seed at `docs/agent/goals/6-server.toml:3737`, both
edited so `goal-switch.py` does not restore the old filing. Both still fail on tests that do
not exist yet, which is this goal's ordinary state.

## Next group

**§ 1's other half, then the member.** One file set: `crates/nvs-server/src/serve.rs`,
`crates/nvs-stdlib/src/socket.rs`, `crates/nvs-host/src/isolate.rs`.

- [ ] **The connection starts it, and the upgrading request ends** — after the join at
      `crates/nvs-server/src/serve.rs:760`, take the slot's other half, and start
      `Isolate::new(program, args, Output::Capture)` from the *connection's* own context
      (`crates/nvs-host/src/isolate.rs:267`), which is what makes it the request's sibling.
      § 1's three `-p nvs-server` tests land here and can be written with no compiler: a
      hand-written request `Program` fills the slot exactly as the member will.
      **Decide the `101` before writing it.** RFC 6455's `Sec-WebSocket-Accept` needs the
      framing crate (`tungstenite`, the goal's standing decision) and nothing can frame a
      byte yet, so answering `101` now would hand a peer a socket no code reads; the safe
      call is to start the isolate, let the request answer its own response, and record in
      the `serve_connection` doc that the status and the socket hand-over are the framing
      slice's. Say which you chose there either way.
- [ ] **The member fills the slot** — both of § 2's entry forms into one `Program`
      (`crates/nvs-stdlib/src/socket.rs:236` is the throwing body,
      `crates/nvs-runtime/src/script.rs:98` is what a `Program` owes its context). A path goes
      through `nvs_runtime::script`'s resolver; a method arrives as the first-class callable
      `nvs_ir`'s `lower_callable` builds, so its program is a closure over that retained value
      plus the request unit's statics recipes and class table. A request with no slot
      (`Inbound::upgrade_slot()` is `None`) is the throw off a CLI program and inside a
      `spawn script` child. §§ 2-3's check is `-p nvs-stdlib` now.
- [ ] **The capability row and the gap list** — the slice that opens the connection owes
      `crates/nvs-stdlib/src/registry.rs`'s `CAPABILITIES` row for reading the entry file,
      which `crates/nvs-stdlib/src/socket.rs:21` says cannot be written ahead of the body.

## Backlog

- `Core\Sse` § 5 and `Core\Topic` § 4 are unregistered — `crates/nvs-stdlib/src/socket.rs`'s
  module doc owns the gap list.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*, narrowed
  by `docs/plan/m7.md`.
- `origin.ignored_address_header()` is ADR 0097 § 6's one `Warn` and still has nowhere to go —
  `crates/nvs-server/src/serve.rs`'s service closure.
