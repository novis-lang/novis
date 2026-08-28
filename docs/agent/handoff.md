# Handoff

## State

**The five catch-all rosters' agreement is complete for `match`.** A
`match` whose subject is a `mixed` compares each label through
`Helper::Identical` (`nvs_ir::lower::expr::lower_match`, whose doc comment
is the rule's home), which is `lower_binary`'s own ADR 0090 § 5 row applied
to a label; the assert that used to panic there is an
internal-consistency check now. The plan's `Open now` carries it and the
three rows the agreement case gained.

- **`tests/conformance/lang/an-operator-agrees-whether-its-operand-is-typed-or-tagged.nvst`
  now reads `agreed=48/48`** — an `int` subject that hits, a `string` one
  that hits, and one that falls off to `default`.
- **An enum `match` subject does not lower, and the typed half is the one
  that fails**: `match ($c) { Mode::Read => … }` over a `Mode $c` is
  *"nvs-codegen does not lower a `Eq` over representation Enum(Int)"*.
  `nvs-ir` reinterprets an enum to its backing integer in
  `lower_literal_membership` but not in `lower_match`; that is the next
  group's first slice.
- **A `foreach` subject that is not iterable was never a hole** — `E0443`
  names ADR 0053 § 3 for a `void` call, an `int`, a `string`, a `bool`, a
  union and an object alike, and a `mixed` is the one deferral. It had no
  case at all; it has one now.
- **A `catch` binding still has no callable members** — `$e->getMessage()`
  panics `nvs-ir` at `lower/expr.rs:2281`. Unchanged.
- **`orient.py`'s pack was complete for this item.** The two standing
  manifest gaps are unchanged — `[context] modules` has no `nvs-runtime`
  and no `nvs-diagnostics` entry.

## Next group

**The enum `match` subject and the two `Ty::Enum` rows under it, over the
one lowering file plus the backend one.** The files:
`crates/nvs-ir/src/lower/expr.rs` and `crates/nvs-codegen/src/emit.rs`.

- [ ] **A `match` over an enum subject** — `crates/nvs-ir/src/lower/expr.rs:1528`
      lowers each label and emits `BinOp::Eq` at the subject's own
      representation, which for a `Mode $c` is `Ty::Enum` and reaches
      `emit_binop`'s representation refusal at
      `crates/nvs-codegen/src/emit.rs:1163`. `Lowering::reinterpret_enum_to_backing`
      (`crates/nvs-ir/src/lower/convert.rs:1194`) is the move
      `lower_literal_membership` already makes for ADR 0047 § 5's identical
      chain — do the same for the subject and each label, once, above the
      chain. `switch` over an enum is worth the same scratch run.
- [ ] **The agreement case gains the enum `match` row** — one line in
      `tests/conformance/lang/an-operator-agrees-whether-its-operand-is-typed-or-tagged.nvst:148`,
      and the count moves 48 → 49. It is written and was removed from this
      session's slice only because the typed half does not run.
- [ ] **`emit_binop`'s enum equality claim, re-read** — its roster at
      `crates/nvs-codegen/src/emit.rs:1163` says equality "is answered for
      … an enum case (through `Reinterpret` to its backing integer, in
      `nvs-ir`)", which is true of `==` and was not of `match`. Check the
      claim against every producer once the slice above lands.

## Backlog

- A `catch` binding's members (`$e->getMessage()`) — `nvs-ir` gap, `lower/expr.rs:2281`.
- `[context] modules` has no `nvs-runtime` and no `nvs-diagnostics` — `docs/agent/loop-goal.toml`.
- A `mixed` `foreach` subject lowers unrefused; nothing pins what it does — ADR 0007 § 2.
- `array<T> as array<U>` inside a `.nvst` still panics on a nested read — playbook.
