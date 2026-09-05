# Handoff

## State

**Goal 6, M7, ADR 0083 § 2 — `Core\Socket::upgrade`'s entry is ADR 0006's operand, enforced.**
`nvs_stdlib::registry::CoreTy::Entry` marks the parameter, `registry::entry_parameter` finds it,
and `nvs_types::expr::isolate`'s `check_core_isolate_call` applies the *same* rule a `spawn script`
gets: `Chat::run(...)` is accepted, an `fn` literal and a `callable` variable are `E0802` with the
construct's own name in the message. Beside it, every non-entry argument now takes ADR 0033 § 4's
crossing refusal — `upgrade(…, args: $secret)` compiled silently before this session.

**The mark is the design call**, made where the item left it open: the row marks itself rather than
the checker keeping a roster, so `Core\Sse` gets the rule in the edit that writes its signature.
`CoreTy::Entry`'s variant doc is the home of why it interns as `mixed` and why the rule cannot be a
parameter type. **The body still throws** — § 1's root isolate is not built — and `Core\Sse` (§ 5)
and `Core\Topic` (§ 4) are still unregistered; `crates/nvs-stdlib/src/socket.rs`'s module doc owns
that gap list.

**ADR 0083 § 2's return bullet is folded**: calling `upgrade` performs it and it answers `void`, the
example no longer `return`s one. That ADR bullet is now the home of the reasoning and `socket.rs`
points at it.

## Next group

**A pre-existing ICE this session's work ran into, over `crates/nvs-ir/src/lower/closure.rs`,
`crates/nvs-ir/src/lower/call.rs` and `tests/conformance/core/`.** It is not ADR 0083's, it is
reachable from ordinary code, and it is what stops the accepted entry form from having a running
`.nvst` case.

- [ ] **A first-class callable naming a `void` method lowers a value nothing defines** — fix
      `lower_callable` at `crates/nvs-ir/src/lower/closure.rs:526`, whose seal is
      `crates/nvs-ir/src/lower/closure.rs:694`; the convention to follow is
      `crates/nvs-ir/src/lower/call.rs:1318` (`(ret != Ty::Void).then(...)`). Decide between sealing
      `Return(None)` for a `Ty::Void` thunk and returning an explicit null — read what
      `nvs_runtime::call_closure` expects back from an `invoke` first, because a `callable` is called
      through it for its value. Reproduce with `mixed $f = C::voidMethod(...);` and nothing else.
- [ ] **The `.nvst` half of ADR 0083 § *Verification*'s "Entry by callable"** — an upgrade whose
      entry is `Chat::run(...)` compiles and reaches the member's own report, which is what says the
      two entry forms arrive at one member. The shape to copy is
      `tests/conformance/core/a-socket-upgrade-reports-the-connection-it-cannot-open-yet.nvst:11`;
      the refusal half already landed as
      `tests/conformance/core/an-upgrade-refuses-an-entry-it-cannot-see-the-function-behind.nvst`.
      Blocked on the item above only because an entry method naturally returns `void`.
- [ ] **A conformance case for the fcc itself, once it lowers** — a `callable` naming a void member
      is called and answers `null`, in `tests/conformance/`; the reference is built at
      `crates/nvs-ir/src/lower/expr.rs:2492` (`lower_callable_ref`).

## Backlog

- The goal's `-p nvs-types` check names `a_secret_fails_to_compile_through_upgrade_args_or_publish`
  (`docs/agent/loop-goal.toml:3771`), a conjunction: the upgrade half landed as
  `a_secret_fails_to_compile_through_upgrade_args` in `crates/nvs-types/tests/isolates.rs`, and the
  `publish` half needs `Core\Topic` (§ 4). Splitting the check is the repair the playbook prescribes.
- `Core\Sse` (§ 5) and `Core\Topic` (§ 4) are unregistered — `crates/nvs-stdlib/src/socket.rs`'s
  module doc owns the list. `CoreTy::Entry` is ready for § 5's entry and a second marked row is what
  `an_entry_parameter_is_declared_only_on_a_static_row` exists to keep honest.
- No `CAPABILITIES` row for `Core\Socket`, owed by the slice that opens the connection — same file's
  module doc.
- `limits:`, `grants:` and `on:` are still not parameters of `upgrade`; they arrive when
  `E_SPAWN_OPTION_UNSUPPORTED` stops refusing them at the sibling site (same module doc).
- **`orient.py` gap:** `[context] modules` names no `nvs-ir` pattern, and the next group is entirely
  in that crate — add one, or the next session reads the map for a crate it is about to edit.
