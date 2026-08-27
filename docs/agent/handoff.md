# Handoff

## State

**M4 — language completeness**, and **Stage 0 (the operator table) is down to its four
named cargo guards** — every operator row itself now runs.
[loop-goal.md](loop-goal.md) is the target, the 43 items and the standing decisions;
[loop-goal.toml](loop-goal.toml) is every check. `python tools/holes.py` is the worklist
and no session re-derives it — `--item N` prints one item's anchors, refusal sites and
cases; `python tools/loop.py --list` prints the named `.mwlt` cases and which are not
written yet.

**Items 1 through 8 are done.** `==` and `!=` over two cases of one enum compile and
answer their backing integers' comparison, for both `EnumRepr`s. The mechanism is
`Lowering::reinterpret_enum_to_backing`, whose own doc comment is its home: the free
`InstKind::Reinterpret` of ADR 0010 § 5 row 1, emitted per operand in `lower_binary`
before the `BinOp` table, so `mwl-codegen` still sees only the `Int`/`Uint`/`Bool`
compare rows its `integral` set names and gains no `Ty::Enum` arm. ADR 0047 § 5's
membership chain now calls that same helper instead of carrying its own copy of the
match. Conformance 560, differential 162, `verify.py` green. No valgrind leg: a
`Reinterpret` transfers no ownership and an enum is not refcounted, so the change adds
no refcount edge.

**Only `==`/`!=` are relabelled, deliberately.** `$a < $b` over two enum cases still
type-checks — `operators.rs`'s result table falls back to `mixed` for a non-numeric pair
— and then fails in `emit_binop` with *"a `Lt` over representation Enum(Int)"*. No ADR
gives ordering two cases a meaning, and ADR 0090 § 2 keeps an enum and its integer in
separate domains on purpose, so relabelling for `<` would have invented a language rule
from inside a codegen fix. It is in `## Backlog` as a refusal to write, not as a
lowering to add.

## Next group

**The four cargo guards that close Stage 0**, then the first item of Stage 1. The first
three share `crates/mwl-types/src/expr/operators.rs`; the fourth is `mwl-ir` and can be
taken with them or alone.

- [ ] **The three `mwl-types` guards item 1 owes**, all still absent:
      `a_mixed_numeric_pair_widens_the_narrower_operand`,
      `a_mixed_numeric_comparison_widens_the_same_way` and
      `an_integer_division_is_a_union_widened_at_its_binding`, over
      `crates/mwl-types/src/expr/operators.rs:106` (`binary_result`) and ADR 0007 § 4's
      table. Named by `docs/agent/loop-goal.toml:205,219`.
- [ ] **The `mwl-ir` guard beside them**, `a_mixed_numeric_pair_converts_before_the_operator`,
      pinning that `Helper::NumericEq`/`NumericLt`/`IntToFloat` are chosen in
      `crates/mwl-ir/src/lower/expr.rs:2299` (`lower_binary`) and never handed to
      `mwl-codegen` as a mismatched-representation `BinOp`. `loop-goal.toml:205,219`.
- [ ] **[10] `break 2` and `continue 2` lower** — `crates/mwl-ir/src/lower/control.rs:1365`
      refuses a non-literal level and `:1375` a multi-level one; `lower/mod.rs:4212` is
      the loop-stack anchor. `python tools/holes.py --item 10`.

## Backlog

- **`<`/`<=`/`>`/`>=`/`<=>` over two enum cases needs a diagnostic**, not a lowering — a
  new `E04xx` in `mwl_types::expr::operators` naming `$e as int` (ADR 0010 § 3), since
  today the pair reaches `emit_binop` and dies there. Decide it under
  `loop-goal.md` § *Standing decisions*' "decide and record".
- [11] a ternary or `match` whose arms lower to two representations — `holes.py --item 11`.
- [16] named and spread call arguments, checker half first — `loop-goal.md`.
- [17] a `...spread` array-literal element, 7 sites — `holes.py --item 17`.
- `Core\Json::decodeAs<T>`'s wider codec-reachable set — `mwl_stdlib::json` gaps.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
