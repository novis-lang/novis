# Handoff

## State

**M4's frontier is Stage 5's remainder: items 32, 33 and 36 of `docs/agent/loop-goal.md`.** ADR 0022
§ 3's never-written storage state landed this session, which closed ADR 0043 § 4 bullet 2 with it —
that ADR has no open bullet left, and item 31 is done whole.

- **The mechanism has one home each and is not restated here**: the storage state is
  `nvs_runtime::Tag::Unset` (`crates/nvs-runtime/src/value.rs:84`), the stamp is
  `FieldDefault::Unset` (`crates/nvs-runtime/src/object.rs:355`) armed by
  `nvs_ir::lower::property_defaults` (`crates/nvs-ir/src/lower/mod.rs:281`), the compiled guard is
  `Lowering::emit_never_written_guard` (`crates/nvs-ir/src/lower/expr.rs:3063`), the forward's is in
  `delegation_forward` (`crates/nvs-ir/src/lower/call.rs:1000`), and the erased read's is in
  `nvs_object_slot_get` (`crates/nvs-runtime/src/object.rs:1943`).
- **`python tools/holes.py` reports no reachable refusal site left**: all nine it still attributes to
  item 1 are the internal-consistency catch-alls the plan already records as having no reachable
  target. The worklist is now the *features* of Stages 5–7, not the panics.
- **`orient.py`'s pack was complete for this item.** `[context] modules` still prints no map line for
  `nvs-runtime`'s `value`/`object`, for `nvs-types`' `check`/`expr_table`/`conformance`, or for
  `nvs-codegen`'s `ty`, all of which this session edited; `[context] adrs` has no ADR 0022 § 3 or
  ADR 0038 § 1 entry, which is the section the item is written against.

## Next group

**Stage 6 item 39's three remaining signature-level gaps, plus the promoted-parameter backlog item
that sits in the same check.** The file set is `crates/nvs-types/src/signatures.rs`,
`crates/nvs-types/src/expr/calls.rs` and `crates/nvs-types/src/expr/iteration.rs`; the next free
diagnostic code is `E0722`.

- [ ] **A visibility keyword on a *non*-constructor method's parameter is refused**, ADR 0043 § 4's
      own backlog line: `Param::is_promoted` is the one home of which parameters promote
      (`crates/nvs-syntax/src/ast.rs:505`) and `record_promoted_properties` only ever asks it of a
      constructor (`crates/nvs-types/src/signatures.rs:880`), so the keyword elsewhere declares
      nothing and is silently ignored where PHP refuses it.
- [ ] **A class with no explicit `constructor` is held to a zero-argument arity check on `new`**,
      item 39: `infer_new` (`crates/nvs-types/src/expr/calls.rs:222`) has the resolved target and
      makes no arity judgement when there is no declaration to resolve against.
- [ ] **A `foreach` *key* binding declared at anything but `string` is refused**, item 39: ADR 0007
      § 5 gives an array one stored key type, so any other annotation is always wrong, and today it
      type-checks and then trips an assertion in `nvs-ir`. `check_foreach_key`
      (`crates/nvs-types/src/expr/iteration.rs:357`).

## Backlog

- Item 32 — ADR 0046 §§ 4–6, `Core\Attributes::get<T>`/`::all<T>` and the call-site `<T>`; three of
  Stage 8's named cases wait on it and on item 33 (`python tools/holes.py --cases`).
- Item 33 — ADR 0092 and `Core\Debug::dump`, with ADR 0033's redaction of a `secret` property.
- Item 36 — a `FATAL` releases the frame's locals; `nvs_ir::ir::Inst::on_error` owns the asymmetry
  (`nvs-codegen` gap 3).
- A user-declared class constant's type at an expression site — item 39's fourth gap, left out of
  the group above because it is `nvs_types::consts`' file rather than these three.
- A `require` whose path is not a string literal runs nothing at all, silently, in both forms —
  `nvs_hir::requires`' own known gap.
- Stage 6 items 37, 38, 40, 41 and 42 — narrowing spellings, reachability, equality-operand
  compatibility, `inout` type agreement, and the unparsed front-end constructs.
