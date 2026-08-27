# Handoff

## State

**M4 — language completeness**, and Stage 0 (the operator table) is two thirds closed.
[loop-goal.md](loop-goal.md) is the target, the 43 items and the standing decisions;
[loop-goal.toml](loop-goal.toml) is every check. `python tools/holes.py` is the worklist and no
session re-derives it — `--item N` prints one item's anchors, refusal sites and cases;
`python tools/loop.py --list` prints the named `.mwlt` cases and which are not written yet.

**Items 1 and 2 are done.** ADR 0007 § 4's promotion table runs: arithmetic widens the integer
side through the same checked helper `$n as float` uses, ordering over a mixed numeric pair takes
the new exact `Helper::NumericLt`/`NumericLtEq` instead (widening there would throw above 2^53
where PHP answers an ordering — recorded in `Helper::NumericLt`'s own doc and in `mwl-ir`'s
known gap 19), § 2's implicit widening now lands at a binding, and integer `/` answers ADR 0007
§ 4's union through `mwl-codegen`'s `emit_int_div`. Four cases landed; conformance 554,
differential 161, `verify.py` green.

**No valgrind run was made.** The new error edges are the shape every fallible helper already
has (`emit_fallible`'s landing block) over scalar operands, so nothing new is refcounted — but a
`valgrind` leg over a `try` around a widening is a cheap confirmation nobody has taken.

## Next group

Three slices, **all three in `emit_binop` and the `BinOp`/`InstKind` enums beside it** — the same
file set the last two used, and the rest of Stage 0. ADR 0007 § 4's table is what specifies all
three; `python tools/holes.py --item 3` and `--item 4` print them in full.

- [ ] **[3] Integer `+`/`-`/`*` throw `ArithmeticError` on overflow.** The three arms wrap today
      and say so in their own comment. The mechanism is next door and now has two users:
      `emit_int_mod` (`crates/mwl-codegen/src/emit.rs:1162`) and `emit_int_div` (`:1234`) each
      raise inline from a baked-in descriptor and take `Inst::on_error`. The arms are
      `crates/mwl-codegen/src/emit.rs:1098`; the lowering has to route them through
      `emit_fallible`, which is `crates/mwl-ir/src/lower/expr.rs`'s `fallible` match just below
      `lower_binary` (`:2271`). Unary `-` at `i64::MIN` is the same question and is at
      `crates/mwl-codegen/src/emit.rs`'s `emit_unop`. Cases:
      `tests/conformance/lang/an-integer-overflow-throws-rather-than-wrapping.mwlt`.
- [ ] **[4] The bitwise operators exist.** `& | ^ << >>` have no `ir::BinOp` variant
      (`crates/mwl-ir/src/ir.rs:1493`) and unary `~` no `InstKind`
      (`crates/mwl-ir/src/ir.rs:248`); the grammar has had all six since M1 and
      `mwl_types::expr::operators::bitwise_result`
      (`crates/mwl-types/src/expr/operators.rs:528`) already types them. Adding the variants gives
      the compound forms free. Note ADR 0007 § 4's asymmetry: `>>` is arithmetic on `int` and
      **logical** on `uint`. Cases:
      `tests/conformance/lang/the-bitwise-operators-answer-over-every-row.mwlt`,
      `tests/differential/lang/the-bitwise-operators-match-phps.mwlt`.
- [ ] **[5] `**` lowers.** `BinaryOp::Pow` reaches `lower_binary`'s `other =>` panic
      (`crates/mwl-ir/src/lower/expr.rs:2271`, the `match op` below the widening) and
      `mwl_types::expr::operators::power_result` already types it. Same overflow rule as [3], so
      it wants that slice's error edge and belongs after it.

## Backlog

- Float `%` does not lower — `emit_binop`'s `match op` has no `fmod` arm, and PHP's `%` converts
  its operands to `int` anyway, so ADR 0007 § 4's "either operand a `float` → `float`" row may be
  wrong for `%` specifically. Decide it when [3] is in the file (`docs/adr/0007-explicit-type-system.md` § 4).
- Ordering over two `string`s still does not lower (`emit_binop`'s ordering set is
  `Int | Uint | Bool`) and `mwl_types` does not refuse it — playbook, *Writing MWL itself*.
- `Core\Reflect::typeOf` does not exist, so no case can name which arm of an `int|float` a
  quotient took except by rendering it (`mwl_stdlib::registry`).
- ADR 0007 § 2's `array<T> as array<U>` row still panics `mwl-ir` — playbook, and the reason
  several `Core` refusals are unreachable from source.
- `python tools/gaps.py`'s `Core` depth slices remain the fallback when a group is blocked
  (`docs/agent/loop-goal.md`).
