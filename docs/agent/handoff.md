# Handoff

## State

**Goal 6, M7 — ADR 0083 § 2's entry is accepted, lowered and pinned; § 1's root isolate is still
the whole of what is missing.** The pre-existing ICE that blocked it is closed:
`nvs_ir::lower::closure`'s `lower_callable` sealed `Return(Some(v))` over a call that defines no
value, so any `Class::voidMethod(...)` — including the `Chat::run(...)` ADR 0083 § 2 accepts —
compiled to an operand nothing defines. A `Ty::Void` thunk now seals `Return(None)`, the seal every
other `void` frame uses, and the caller reads the `null` `nvs_runtime::abi::call` pre-set the out
slot with. `lower_callable`'s own doc comment is the home of why that rather than an explicit
`ConstNull`.

**`Core\Socket::upgrade`'s body still throws** — `crates/nvs-stdlib/src/socket.rs:150` — and that
module's doc owns the gap list (`Core\Sse` § 5 and `Core\Topic` § 4 are still unregistered).

**The driver's failing check cannot pass where it is filed.** `nvs-server (ADR 0083 §§ 1-3)` runs
`cargo test -p nvs-server`, and `crates/nvs-server/Cargo.toml` names `hyper`, `nvs-host`,
`nvs-config`, `nvs-runtime` and `jiff` — **not `nvs-stdlib`**, where every one of the eight named
tests' subject (`Core\Socket::upgrade`, `receive`, `send`, `Core\Sse`) lives; the crate also
inherits the workspace's `unsafe_code = "forbid"`, so no hand-built closure or class table can be
fixtured there either. This is the playbook's "a `-p <crate>` check is *impossible* rather than
unwritten" trap, and the repair is the check's `args`. It is left unmade on purpose: which crate
can host `an_upgrade_opens_a_root_isolate_and_the_upgrading_request_ends` depends on what § 1's
isolate is built over, so the session that builds it should file the tests and correct the check in
the same slice.

## Next group

**§ 1's root isolate — what `Core\Socket::upgrade` does instead of throwing.** One file set:
`crates/nvs-stdlib/src/socket.rs`, `crates/nvs-host/src/isolate.rs`, `docs/agent/loop-goal.toml`.

- [ ] **Decide what an upgrade spawns, and over what** — read `nvs_host::Isolate`'s builder at
      `crates/nvs-host/src/isolate.rs:130` (`new`), `:151`
      (`running_a_method_of_the_parents_unit`, which is ADR 0006's static-method entry) and `:231`
      (`start`, the one that hands back a `Running` rather than joining), against the throwing body
      at `crates/nvs-stdlib/src/socket.rs:150`. ADR 0083 § 1's claim is that the *upgrading
      request's* arena is released while the connection is open, so what the member hands the
      connection to has to outlive the frame that called it: `start` is the shape, `run` is not.
- [ ] **Build it, and make the upgrading request end** — the same two anchors,
      `crates/nvs-stdlib/src/socket.rs:150` and `crates/nvs-host/src/isolate.rs:231`. Both entry
      spellings already reach this body (a path, and `Chat::run(...)` since this session), so the
      member has its operand and nothing above it needs a change.
- [ ] **File §§ 1-3's tests where they can run, and correct the check's `args`** — the block is
      `docs/agent/loop-goal.toml:3738`, and the same block in `docs/agent/goals/6-server.toml` or
      the next `goal-switch.py` restores the impossible filing. `crates/nvs-stdlib/src/socket.rs:150`
      is where the first three name their subject.

## Backlog

- `Core\Sse` (§ 5) and `Core\Topic` (§ 4) are unregistered — `crates/nvs-stdlib/src/socket.rs`'s
  module doc owns the gap list.
- § 3's one wait over two sources — `receive`/`send` have no rows yet; ADR 0083 § 3.
- ADR 0083 § *Verification*'s "Entry by callable" second half — the state-bleed suite cannot tell
  the two entry forms apart — needs § 1 first.
