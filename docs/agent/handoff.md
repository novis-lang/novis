# Handoff

## State

**M4 — language completeness**, and Stage 0 (the operator table) is down to its last three
items. [loop-goal.md](loop-goal.md) is the target, the 43 items and the standing decisions;
[loop-goal.toml](loop-goal.toml) is every check. `python tools/holes.py` is the worklist and no
session re-derives it — `--item N` prints one item's anchors, refusal sites and cases;
`python tools/loop.py --list` prints the named `.mwlt` cases and which are not written yet.

**Items 1 through 4 are done.** Integer `+`, `-`, `*` and unary `-` now throw `ArithmeticError`
on overflow: `mwl-codegen`'s `emit_checked_int_arith` reads Cranelift's `sadd_overflow` family
(the flag the CPU already sets, so the cost is one predicted branch) and raises through the new
shared `raise_arithmetic_error`, which the two zero-divisor guards use too. All six bitwise
operators lower, with PHP's two shift rules that the machine does not give — a negative count
throws, a count past 64 answers all-zeros or all-sign — in `emit_shift`; the five compound forms
came free through `lower_compound_assignment`'s rewrite. Conformance 556, differential 162,
`verify.py` green, and a `valgrind` leg over a fixture that throws from every new edge with a
refcounted local live reports 0 failures.

**The two slices landed as one commit.** They edit the same lines of `emit_binop`, `lower_binary`
and the `BinOp`/`UnOp` enums, so there is no file split that leaves both commits building; the
message names both.

**`orient.py` did not print two files this session needed**: `crates/mwl-ir/src/print.rs` (a new
`BinOp`/`UnOp` variant needs a name there or the crate does not compile) and
`benches/abi-probe/tests/perf_guards.rs`. Add both to `[context] modules` in `loop-goal.toml`.

## Next group

Three slices, **all three in `lower_binary`/`emit_binop` and the enums beside them** — the same
file set the last four used, and the rest of Stage 0. `python tools/holes.py --item 5`, `--item 6`
and `--item 7` print them in full. Shared anchors: `crates/mwl-ir/src/ir.rs:1496` (`BinOp`),
`:1552` (`UnOp`), `crates/mwl-ir/src/lower/expr.rs:2283` (`lower_binary`), `:2092` (`lower_unary`),
`crates/mwl-codegen/src/emit.rs:1009` (`emit_binop`), `:1486` (`emit_unop`),
`crates/mwl-ir/src/print.rs:395` (`bin_op_name`).

- [ ] **[5] `**` lowers**, over every numeric row but the `decimal` base ADR 0054 § 3 refuses.
      `mwl_types::expr::operators::power_result` (`crates/mwl-types/src/expr/operators.rs:577`)
      already types it, so this is an `ir::BinOp::Pow` plus two codegen rows. **Neither row is one
      instruction**: Cranelift has no `ipow` and no `fpow`, so the integer row is a
      square-and-multiply loop that checks overflow at each step (ADR 0007 § 4 lists `**` beside
      `+ - *`, so it throws) and the float row is a call — decide between a `libcall` and an
      `ir::Helper` and record it in `mwl-codegen`'s module doc. `**=` comes free.
      Cases: `a_power_operator_lowers_over_every_numeric_row` (`-p mwl-ir`),
      `tests/conformance/lang/a-power-operator-answers-every-numeric-row.mwlt`.
- [ ] **[6] `<=>` answers for a scalar.** Only ADR 0013's *object* form lowers today; every scalar
      operand reaches `lower_binary`'s `other =>` panic. `object_comparison_result` is the shape to
      match — `-1`/`0`/`1` — and the mixed numeric pair must reuse `Helper::NumericLt`/`NumericLtEq`
      rather than widening, for the reason `lower_binary`'s ordering arm
      (`crates/mwl-ir/src/lower/expr.rs:2432`) already states. Case:
      `a_spaceship_answers_minus_one_zero_or_one_for_a_scalar` (`-p mwl-ir`).
- [ ] **[7] `$x++` and `--$x` lower in either position**, and the compound assignment reads its
      target once. `lower_compound_assignment` (`crates/mwl-ir/src/lower/stmt.rs:331`) rewrites
      `$x op= e` into `$x = $x op e`, which reads the target twice, so `is_reevaluable_target`
      (`:706`) refuses `f()->count += 1`; closing both means splitting the target's address
      computation out of `lower_reassignment` (`:415`). An increment needs that split anyway — its
      `1` has no span to build an AST node from. Cases: `an_increment_lowers_in_either_position`,
      `a_compound_assignment_evaluates_its_target_once` (both `-p mwl-ir`).

## Backlog

- **Items 1 and 2 never wrote their four named cargo guards** and `loop.py` will fail on them:
  `a_mixed_numeric_pair_widens_the_narrower_operand`, `a_mixed_numeric_comparison_widens_the_same_way`
  and `an_integer_division_is_a_union_widened_at_its_binding` in `-p mwl-types`, and
  `a_mixed_numeric_pair_converts_before_the_operator` in `-p mwl-ir`. Item 2's `-p mwl-codegen`
  one is written now. `docs/agent/loop-goal.toml:206-233`.
- `examples/operators.mwl` (Stage 2) goes green only when items 5, 6 and 7 land — `pow=1048576`,
  `cmp=-1,0,1` and `counted=5` are theirs. Its `guarded=overflow` and `bits=22`/`not=-1` rows pass.
- `==` over two enum values still has no `Ty::Enum` arm in `emit_binop` (item 8) —
  `mwl-codegen`'s known gap 9 now says only that.
- `crates/mwl-codegen/src/ty.rs:116` and `:121` are the two refusal sites `holes.py` reports as
  unattributed to any item.
- ADR 0007 § 2's `array<T> as array<U>` still panics `mwl-ir`, which is what blocks several
  `Core` depth cases (playbook, *Writing a test case*).
