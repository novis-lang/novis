# Handoff

## State

**M4's frontier is Stage 5's remainder: item 33 of `docs/agent/loop-goal.md`.** Item 36 landed
whole this session and `nvs-codegen`'s known-gaps list has no gap 3 any more.

- **The rule has one home**: `nvs_ir::ir::Inst::on_error` — *every* instruction returning a status
  carries a landing block, whether or not a `catch` can act on the status, because a `FATAL` and an
  `EXITED` leave the frame too. `Terminator::Catch`'s new `onward` field
  (`crates/nvs-ir/src/ir.rs:2201`) is the `try`-protected half, built in
  `Lowering::landing_block` (`crates/nvs-ir/src/lower/mod.rs:1811`).
- **Both of `nvs-codegen`'s `on_error: None` arms are internal-consistency checks now** —
  `emit_status_check` (`crates/nvs-codegen/src/emit.rs:2934`) and `raise_arithmetic_error`
  (`crates/nvs-codegen/src/emit.rs:1827`). An arrival at either is a `nvs-ir` site that used the
  plain `Lowering::emit` for a status-returning kind; that refusal is what enumerates them — four
  producers, two of them in ADR 0053 § 4's synthesized generator frames — and the playbook's new
  bullet says why a grep does not.
- **A closed `nvs-codegen` gap is deleted and its number retired**, stated at the head of that
  crate's known-gaps list. The list now reads 1, 2, 4 … 9 on purpose.
- **`orient.py`'s pack was complete for this item.** No `[context]` field was missing.

## Next group

**One item, and it is a whole feature rather than a slice over landed work — expect it to fill a
session on its own.** Read its own line in `docs/agent/loop-goal.md` first; it is not in
`[context]`.

- [ ] **Item 33 — ADR 0092 and `Core\Debug::dump`**, including ADR 0033's redaction of a
      `secret`-qualified property, which that ADR's own *Verification* defers to M4 by name. The
      file set is `crates/nvs-stdlib/src/registry.rs` plus a new member module beside it, and the
      qualifier is read where `nvs_types::expr::quals` already reads one. ADR 0092 is unread by any
      session so far — `python tools/peek.py docs/adr/0092-*.md:"## 1"` before anything else, and
      add its sections to `[context] adrs`.
- [ ] **Then Stage 6 opens** (items 37–40, `crates/nvs-types/src/locals.rs` and
      `expr/operators.rs`); item 37's three narrowing spellings and item 40's equality-operand pass
      share `nvs-types` and are the first pair that genuinely shares a file set.

## Backlog

- A `require` whose path is not a string literal runs nothing at all, silently, in both forms —
  `nvs_hir::requires`' own known gap.
- `nvs-ir`'s remaining inline-release producers (a normalized subscript key, a `match` subject) leak
  on a throwing edge — `Lowering::landing_block`'s own *Known gap*.
- `nvs-codegen` gap 4: two identical string literals are two data objects.
- ADR 0043 § 4's `E_DELEGATE_TYPE_MISMATCH` is not built — `resolve_delegations`' doc comment.
- ADR 0018's `BRANCH` probe (`nvs-codegen` gap 2) stays out of scope, per the goal's standing
  decisions.
