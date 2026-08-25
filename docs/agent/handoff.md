# Handoff

## State

**Stage 0 items 1-6, 7a, 7b and 9 are done; 7c is untouched and item 8 is untouched.**
`instanceof` against a compiler-declared global interface now runs end to end.
[`mwl_types::layout::build_class_layouts`](../../crates/mwl-types/src/layout.rs#L191) seeds a layout for
every `mwl_hir::interfaces::RESERVED` name exactly the way it already seeds `mwl_hir::errors`' exception
tree, so `mwl-codegen` has a `ClassDesc` to bake in and — the half that actually mattered — an
implementor's `conforms` list keeps its edge, since `Classes::define` drops any label the unit declares no
class for. `mwl_types::expr::members::infer_instanceof` records the resolved class for a reserved
interface too. Consequences pinned by
`tests/conformance/class/instanceof-answers-for-a-reserved-global-interface.mwlt`: `$m instanceof
Stringable` is true for an implementor, true for a subclass that names nothing, false for an unrelated
class, and true for a generator's synthesized state machine against `Iterator`.

Cost: four extra `ClassDesc`s per compiled unit, no fields and no methods on any of them.

`python tools/verify.py` is green (1370 tests) and `mwl test tests/` is 430/0.

## Next group — item 7c, which is two slices rather than one, then item 8

**Shared file set:** `crates/mwl-types/src/expr/operators.rs`, `crates/mwl-hir/src/hierarchy.rs`,
`crates/mwl-ir/src/lower/expr.rs` and `tests/conformance/class/`. The rule is
[`loop-goal.md`](loop-goal.md) § *Stage 0* items 7 and 8.

- [ ] **7c-i — the implicit `toString` desugar, `mwl-ir` gap 12
      ([lib.rs:218](../../crates/mwl-ir/src/lib.rs#L218)).** Do this one **first**: `echo $m` on a
      *concrete* `Stringable` implementor already type-checks and then panics at
      [expr.rs:436](../../crates/mwl-ir/src/lower/expr.rs#L436), so widening the checker first would only
      move 7c's refusal from a diagnostic to a panic. The gap's own text names the two ways out — the
      checker records an `ExprInfo::Call` for the operand's `toString()`, or this crate re-resolves it —
      and the first keeps `mwl-ir`'s "trusts its input" rule. `.`, `as string` and `echo` share the
      conversion, so one desugar closes all three.
- [ ] **7c-ii — a value typed at the interface itself.** `require_stringable`
      ([operators.rs:792](../../crates/mwl-types/src/expr/operators.rs#L792)) and
      `object_comparison_result` ([operators.rs:329](../../crates/mwl-types/src/expr/operators.rs#L329))
      both ask `mwl_hir::implements_interface`
      ([hierarchy.rs:367](../../crates/mwl-hir/src/hierarchy.rs#L367)), which walks *parents* only and so
      answers `false` for `Stringable` against `Stringable`. Its other two callers (`class_satisfied`,
      `classes_are_unrelated`) already return on equal names before calling, so making the walk reflexive
      is a one-place decision — but say which you chose in that function's doc. Then the `.mwlt` cases item
      7 asks for: a `Stringable` parameter echoed, and a `Comparable` one ordered through `<`. Check the
      second actually *lowers* before writing it; a `<` over two interface-typed operands has no single
      resolved `compareTo` to call.
- [ ] **8 — ADR 0061's `autoload`**, both file-scope declaration forms and the name-to-file fixpoint over
      the require-graph worklist. `loop-goal.toml` already waits on `an_autoload_declaration_parses`
      (`mwl-syntax`) and `an_autoload_declaration_resolves_a_name_to_its_file` (`mwl-hir`). Different file
      set from 7c — start a session on it rather than tacking it onto one.

## Backlog

- `mwl-ir` gap 21 (new): a binding declared at the opaque `object` top has no representation arm, so
  `object $o = $obj;` panics — that gap's text says what a session landing it owes.
- ADR 0094's one uncovered shape: a promoted constructor parameter is no table's property, so nothing
  resolves it to check — `mwl-types`' `signatures` known gaps.
- `private(set)` is ADR 0094 § 3's write half and is not modeled at all — same known-gap list.
- ADR 0043 § 4's `by $field` delegation exempts a whole class from conformance — `mwl-types`'
  `conformance` module doc.
- No override-compatibility check exists, so ADR 0013 § 1's `compareTo(self)` variance rule is unenforced
  — `mwl-types`' `signatures`.
- `mwl-ir` gap 19: `$n + $f` and `$n < $f` type-check and still fail in codegen.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
