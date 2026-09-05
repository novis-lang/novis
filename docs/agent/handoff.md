# Handoff

## State

**Goal 6, M7 — ADR 0083 § 1's design call is made and recorded; nothing of § 1 is built
yet.** The decision is that `Core\Socket::upgrade` **spawns nothing**: it prepares an
isolate — a `nvs_runtime::script::Program` and the already-crossed `args` — and records it
on the request carrier, and the *connection* starts it from its own context after the
request has been joined and its context dropped. Its one home is
`crates/nvs-stdlib/src/socket.rs`'s module doc, § *Decision: this member spawns nothing,
and the connection starts it* (`crates/nvs-stdlib/src/socket.rs:37`), which owns why the
member cannot start it, why all three refusals (capability, argument, code) belong to the
request, what the carrier choice buys, and what the second graph copy spends.
`crates/nvs-host/src/isolate.rs`'s module doc names the connection as `Isolate::start`'s
third caller; **`nvs_host::Isolate` needs nothing added** — no new builder, no new
`Charge`, no new `Entry`.

**The body still throws** — `crates/nvs-stdlib/src/socket.rs:231` — and that module's doc
owns the gap list (`Core\Sse` § 5 and `Core\Topic` § 4 are still unregistered).

**The driver's failing check still cannot pass where it is filed, and the decision changes
the repair.** The previous triage — that every one of the eight named tests needs
`nvs-stdlib`, which `crates/nvs-server/Cargo.toml` does not name — holds for §§ 2-3's four
and **not** for § 1's three: what crosses from the request to the connection is a
`Program`, a boxed Rust closure over `nvs-runtime` types, so a `-p nvs-server` test builds
one by hand with no Novis compiler and no `unsafe` (the crate forbids it). So the check
**splits** rather than moves: § 1 and § 5 stay `-p nvs-server`, §§ 2-3 become a second
`cargo-named` check under `-p nvs-stdlib`.

## Next group

**§ 1's connection, built in the order it can be tested in.** One file set:
`crates/nvs-runtime/src/ctx/inbound.rs`, `crates/nvs-server/src/serve.rs`,
`crates/nvs-stdlib/src/socket.rs`, `docs/agent/loop-goal.toml`.

- [ ] **The carrier, and the door's offer** — an `Upgrade` (program, crossed args) and the
      slot a request writes it into, on `nvs_runtime::Inbound`
      (`crates/nvs-runtime/src/ctx/inbound.rs:113` is the struct,
      `crates/nvs-runtime/src/ctx/inbound.rs:23` is `set_inbound`), offered by the door
      only for a request whose connection can be upgraded — the server stashes `hyper`'s
      `OnUpgrade` beside it, which is why the slot is a shared cell the connection keeps
      the other half of (`crates/nvs-server/src/serve.rs:637`). A request with no slot is
      what makes the member throw off a CLI program and inside a `spawn script` child.
- [ ] **The connection starts it, and the upgrading request ends** — after
      `crates/nvs-server/src/serve.rs:665`'s join, drop the request's context, write the
      `101`, then `Isolate::start` from the connection's own context
      (`crates/nvs-host/src/isolate.rs:239`). § 1's three tests land here under
      `-p nvs-server`, and the ordering is what
      `the_upgrading_requests_arena_is_released_while_the_connection_is_open` asserts.
- [ ] **The member fills the slot** — both of § 2's entry forms into one `Program`
      (`crates/nvs-stdlib/src/socket.rs:231` is the throwing body,
      `crates/nvs-runtime/src/script.rs:98` is what a `Program` owes its context). A path
      goes through `nvs_runtime::script`'s resolver; a method arrives as the first-class
      callable `nvs_ir`'s `lower_callable` builds, so its program is a closure over that
      retained value plus the request unit's statics recipes and class table.
- [ ] **Split the failing check** — `docs/agent/loop-goal.toml:3738` keeps § 1's three and
      § 5's one under `-p nvs-server`; §§ 2-3's four move to a second `cargo-named` check
      under `-p nvs-stdlib`. Fix `docs/agent/goals/6-server.toml` in the same edit or the
      next `goal-switch.py` restores the old one.

## Backlog

- § 3's `receive`/`send` need the framing crate; `tungstenite` is pre-authorized in
  `docs/agent/loop-goal.md` § *Standing decisions*.
- § 5's `Core\Sse` and § 4's `Core\Topic` are unregistered —
  `crates/nvs-stdlib/src/socket.rs`'s module doc owns the gap list.
- What an upgrading request's echoed bytes mean (there is no body in a `101`) is undecided;
  ADR 0088 § 3's table has no row for a connection isolate either.
