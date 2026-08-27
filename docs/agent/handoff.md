# Handoff

## State

**M4 — language completeness**, and Stage 0 (the operator table) is down to **one** item:
`==` over two enum values. [loop-goal.md](loop-goal.md) is the target, the 43 items and
the standing decisions; [loop-goal.toml](loop-goal.toml) is every check.
`python tools/holes.py` is the worklist and no session re-derives it — `--item N` prints
one item's anchors, refusal sites and cases; `python tools/loop.py --list` prints the
named `.mwlt` cases and which are not written yet.

**Items 1 through 7 are done.** `$x++` and `--$x` lower in either position, as statements
and as a `for` step, over a local, a property and an array element at every numeric
representation — the prefix/postfix split decides only which value a surrounding
*expression* sees, so both spellings are one call to `lower_incdec_stmt`. The compound
forms that inherited the hole came with them: `lower_read_modify_write` computes the
target's **address** before the `$t = $t ⊕ e` rewrite is built, so `Box::make()->count += 1`
calls `make()` once. Conformance 559, differential 162, `verify.py` green, and a `valgrind`
leg over a fixture that runs a staged refcounted receiver, a string append through one, and
a throwing increment with both in flight reports 0 failures.

**The mechanism is `Lowering::staged_targets`**, whose own doc comment is its home:
`(span, value, ty)` entries that `lower_expr` answers from before it looks at an
expression's kind, and that `aliasing_read` calls borrows. `lower_reassignment` split into
itself plus `lower_store` taking a `Stored::{Expr,Value}`, which is what lets a
read-modify-write hand over a value it has already combined; the playbook's new Tooling
bullet says why no synthesized `ExprKind::Variable` could have done this instead.

**One decision taken and recorded** (under § *Standing decisions*' "decide and record",
not an ADR): **PHP's string increment does not exist** — `$s++` is E0474, because ADR 0007
§ 2 fixes a binding's type and § 4's table has no row producing `"b"` from a `string` and a
`1`. `mwl_types::expr::operators`' module doc is the home. A target `mwl_types` cannot pin
down (a `mixed`, a `?int`) is *not* refused there: its `1` is emitted as an `int` so it
lands in the same place, with the same message, that `$m += 1` already lands in.

**An increment used as a value** (`$y = $x++;`) is deliberately still unlowered: it is the
same hole a nested assignment (`$y = ($x = 5);`) is, and for the same reason — `lower_expr`
is handed an `&Env` and has no binding to re-point. See `lower_incdec_stmt`'s doc comment.

## Next group

**The end of Stage 0.** The first two share `crates/mwl-codegen`; the third is the
`mwl_types`/`mwl-ir` half of items 1 and 2's guards and can be taken alone.

- [ ] **[8] `==` over two enum values compiles** — `emit_binop`'s `integral` set is
      `Int | Uint | Bool`, so an `Eq` over `Ty::Enum(Int)` fails with *"a `Eq` over
      representation Enum(Int)"*. Compare one representation down:
      `InstKind::Reinterpret` to the backing integer is free and is the row `$m as int`
      already uses. `crates/mwl-codegen/src/emit.rs:1008`. ADR 0090 § 2 (an enum is its own
      equality domain) and ADR 0010 § 3. `python tools/holes.py --item 8`.
- [ ] **The guard it owes**, `two_enum_values_compare_as_their_backing_integer`, in
      `crates/mwl-codegen/tests/arithmetic.rs` beside `an_integer_addition_traps_on_overflow`.
      Named by `docs/agent/loop-goal.toml:240`.
- [ ] **The four cargo guards items 1 and 2 never wrote**, all still absent:
      `a_mixed_numeric_pair_widens_the_narrower_operand`,
      `a_mixed_numeric_comparison_widens_the_same_way` and
      `an_integer_division_is_a_union_widened_at_its_binding` in `mwl-types`
      (`crates/mwl-types/src/expr/operators.rs:96` `binary_result`, `:403`), and
      `a_mixed_numeric_pair_converts_before_the_operator` in `mwl-ir`
      (`crates/mwl-ir/src/lower/expr.rs:2291` `lower_binary`). `loop-goal.toml:205,219`.

## Backlog

- An increment or an assignment **in an expression position** — one hole, two spellings;
  `mwl_ir::lower::stmt`'s `lower_incdec_stmt` doc comment says why.
- `$m++`/`$m += 1` over a `mixed` or a `?int` still ends at *"mismatched representations"*
  rather than a rule-naming diagnostic — item 11's neighbourhood, `mwl-codegen` gap.
- A **static property** assignment target (`Box::$calls = …`) does not lower;
  `crates/mwl-ir/src/lower/stmt.rs`'s reassignment panic names it.
- `holes.py` attributes stmt.rs's five surviving refusals to item 7 by file; they are the
  nullsafe target, the erased receiver, the erased array base and `unset`'s — other items'.
- ADRs 0091, 0093, 0097 and 0100 § 3 stay out of scope (M6–M10), per § *Standing decisions*.
