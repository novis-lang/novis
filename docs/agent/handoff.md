# Handoff

## State

**ADR 0007 § 4's ordering row is closed at both ends.** `<`/`<=`/`>`/`>=`/`<=>`
over an operand that table gives no row for is `E0715` where it is written
(`mwl_types::expr::operators::reject_unordered_operand`), `bool` and
`null == null` are rows rather than refusals, and the object family keeps
`E0411`. The rule's one home is ADR 0007 § 4's own table plus the closure
paragraph under it; the code's is `code::E_ORDERING_HAS_NO_ROW`'s doc comment.

- **Items 4, 1 and 8 of the goal's Stage 0 were already landed when this session
  opened** — the bitwise operators, the promotion table and enum `==` all run,
  and `holes.py` attributes their remaining "refusal sites" to catch-alls the
  plan has already proved unreachable. What was genuinely open behind item 1 was
  `emit_binop`'s *representation* catch-all, and that is what this session took.
- **`emit_binop`'s catch-all now carries its own roster**, the way the statement
  and expression dispatches do: what is left is `Ty::Tagged` (item 24) and the
  three representations no source expression has. A future session that wants to
  delete the site itself needs item 24 first.
- **`orient.py`'s pack was complete for this item.** The two gaps the previous
  handoff named are still open: `[context] modules` has no `mwl-runtime` entry,
  and it has no `mwl-diagnostics` one either — this session read
  `crates/mwl-diagnostics/src/lib.rs` around the `E07xx` band to claim `E0715`.

## Next group

**The remaining representation catch-alls, which is `mwl-codegen`'s `emit.rs`
one more time, then `mwl-ir`'s erasure map.** The file set:
`crates/mwl-codegen/src/emit.rs`, then `crates/mwl-ir/src/lower/mod.rs`.
`python tools/holes.py --item N` prints any of these in full.

- [ ] **Item 25, `object` as a declared type** — `erase_checked_ty` reaches no
      arm for ADR 0007 § 3's `object` top, so `object $o = $obj;` panics.
      `crates/mwl-ir/src/lower/mod.rs:2206`. The arm is one line; what the slice
      owes is the check that nothing below reads a class *label* off an operand
      it would now receive without one — `ClassDesc::renderer` and
      `ClassDesc::unwind` are the two descriptor fields a release or a rendering
      already reads, and both are found from the instance rather than from the
      static type, which is the argument to write down.
- [ ] **Item 24's first half, a tagged operand under a comparison** — the one
      target `emit_binop`'s catch-all has left. `mwl_runtime::value_truthy` is
      the precedent for the shape: an `ir::Helper` variant dispatching on the
      tag, never a second representation. `crates/mwl-ir/src/ir.rs:1180`
      (`Helper`), `crates/mwl-codegen/src/emit.rs:1134` (the catch-all's roster).
- [ ] **`emit.rs`'s six remaining internal panics** — `reinterpret`, the tagged
      widen/narrow pair, the unary catch-all, the refcount one, the terminator
      one and the runtime-helper one. Each is an internal-consistency check
      rather than a hole; what a session owes is the roster comment, in the shape
      `emit_binop`'s now has. `crates/mwl-codegen/src/emit.rs:660`, `:690`,
      `:721`, `:1795`, `:2787`, `:2938`, `:3295`.

## Backlog

- A throw escaping an abandoned generator's `finally` is discarded — ADR 0028 § 2,
  behind ADR 0020's ladder.
- `[context] modules` has no `mwl-runtime` or `mwl-diagnostics` entry —
  `docs/agent/loop-goal.toml`.
- `string as Core\Html\Markup` is `mwl-ir`'s one remaining conversion catch-all
  target and waits on `Core\Html` at M7 — ADR 0024 § 5.
- Item 17's `&value` array element still owes its refusal —
  `docs/agent/loop-goal.md` § *Standing decisions*.
- `void` on either side of a comparison fails in `mwl-ir` with "an operand used
  before it is defined" rather than a diagnostic — `mwl-ir`'s known gaps.
