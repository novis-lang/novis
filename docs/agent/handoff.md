# Handoff

## State

**M4 — language completeness**, and Stage 0 (the operator table) is down to its last two
items. [loop-goal.md](loop-goal.md) is the target, the 43 items and the standing decisions;
[loop-goal.toml](loop-goal.toml) is every check. `python tools/holes.py` is the worklist and no
session re-derives it — `--item N` prints one item's anchors, refusal sites and cases;
`python tools/loop.py --list` prints the named `.mwlt` cases and which are not written yet.

**Items 1 through 6 are done.** `**` lowers over every numeric row: the integer one is a
square-and-multiply loop in `emit_int_pow` that checks overflow at each step and throws
`ArithmeticError`, including for a negative exponent (ADR 0007 § 4's row is "no promotion to
`float`", so `2 ** -1` has no `int` to answer — a base of `1` or `-1` does, and is returned),
and the float one calls the new `mwl_runtime::mwl_float_pow`. `<=>` answers `-1`/`0`/`1` for
every scalar, by three routes that a swept test asserts *agree*: inline `BinOp::Cmp` for a
matched pair, `Helper::NumericCmp` for a mixed numeric one, `Helper::DecimalCmp` for a
`decimal`. Conformance 558, differential 162, `verify.py` green, and a `valgrind` leg over a
fixture holding a refcounted local across all three of `**`'s new throwing edges reports 0
failures.

**Two decisions this session took and recorded** (both under § *Standing decisions*' "decide
and record", neither an ADR): `mwl_float_pow` is a **direct** two-`f64` call and not an
`mwl_ir::Helper`, because Cranelift has no `fpow` and no `LibCall::Pow` so the row must be
*some* call — `mwl-codegen`'s `emit` module doc § *A runtime call that is not a helper* is the
home. And `<=>` answers **`1`** for an unordered pair, not `0`, which is PHP's own answer and
the reason the emission is "less, else equal, else 1" rather than `(a > b) - (a < b)`;
`mwl_runtime::helpers::spaceship` is the home.

**The two slices landed as one commit.** They edit the same lines of `emit_binop`, the
`BinOp`/`Helper` enums and `lower_binary`'s table, so no file split leaves both commits
building; the message names both. That is the third group in a row this has been true of, and
it is a property of the operator table rather than of any one session.

## Next group

**The last two items of Stage 0**, and they do *not* share a file set — take item 7 first and
alone, since it is the larger and the only one in `stmt.rs`. `python tools/holes.py --item 7`
and `--item 8` print them in full.

- [ ] **[7] `$x++` and `--$x` lower in either position**, and with them the compound forms that
      inherit the hole. Two things split out of `lower_reassignment` first: the target's
      *address* computation, so `f()->count += 1` evaluates `f()` once where the rewrite reads
      it twice, and the increment's `1`, which has no source span to build an `ExprKind::Int`
      from. `crates/mwl-ir/src/lower/stmt.rs:415` (`lower_reassignment`), `:706`
      (`is_reevaluable_target`), `:331` (`lower_compound_assignment`). `mwl-ir` gap 16. Cases:
      `an_increment_lowers_in_either_position` and
      `a_compound_assignment_evaluates_its_target_once`,
      `tests/conformance/lang/an-increment-answers-the-same-in-either-position.mwlt`.
- [ ] **[8] `==` over two enum values compiles.** `Ty::Enum(Int)` is not on `emit_binop`'s
      `integral` list, so the comparison `mwl-ir` already lowers is refused in the backend; the
      enum-to-backing `InstKind::Reinterpret` that `lower_literal_membership`
      (`crates/mwl-ir/src/lower/expr.rs:3955`) does for a membership chain is the same move.
      `crates/mwl-codegen/src/emit.rs:1031` (`emit_binop`). `mwl-codegen` gap 9's second shape.

Shared anchors for item 8 and anything else in the operator table:
`crates/mwl-ir/src/ir.rs:1509` (`BinOp`), `:1589` (`UnOp`),
`crates/mwl-ir/src/lower/expr.rs:2291` (`lower_binary`), `:2100` (`lower_unary`),
`crates/mwl-codegen/src/emit.rs:1031` (`emit_binop`), `:1524` (`emit_int_pow`).

## Backlog

- `<` and `<=>` over two `string`s still do not lower — `emit_binop`'s `integral` set is
  `Int | Uint | Bool`, so `<=>` inherits the existing hole and refuses with
  *"does not lower a `Cmp` over representation Str"*. Graceful, not a panic, and the same site
  as `<`'s. Owned by `mwl-codegen`'s own gap list; the playbook bullet describes it for cases.
- `orient.py` printed everything this session needed once `crates/mwl-ir/src/print.rs` was in
  `[context] modules` — the previous handoff asked for it and for
  `benches/abi-probe/tests/perf_guards.rs`; add `crates/mwl-codegen/tests/*.rs` too, since
  item 6's named cargo case lives there and nothing in the pack said so.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
- Conformance is 558 of the goal's 750; `python tools/gaps.py` ranks the thin `Core` classes
  when a language group is blocked.
