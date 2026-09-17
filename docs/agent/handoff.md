# Handoff

## State

**Goal `core-class-tests` is met.** A `Core` class with instances is a written class name all three
type tests walk, against the process-wide descriptor `nvs_stdlib::class_descriptors` publishes:
`$v is Core\Time\Date`, `$v instanceof Core\Time\Date` and `$v as Core\Time\Date` compile, answer at
run time and narrow the subject on the true edge. `verify.py` is 11 of 11 green — 4748 tests, 2024
conformance cases.

What moved this session, one sentence each. `nvs_types::expr::operators`'s
`reject_unrelated_class_conversion` (`crates/nvs-types/src/expr/operators.rs:1853`) now sends only a
`Core` **namespace** class to `E0711` and asks every other `Core` class the same disjointness question a
declared class gets; `testable_core_class` (`crates/nvs-types/src/expr/members.rs:435`) is `pub` so
`nvs-ir` asks the one roster rather than a copy; and `declared_class`
(`crates/nvs-ir/src/lower/closure.rs:366`) answers for that roster, which is all
`lower_checked_downcast` needed — `InstKind::InstanceOf` already carried the name and `emit_instanceof`
already relocated it.

**Still refused, on purpose**: a `Core` namespace class as a test or a conversion target (`E0496`,
`E0711`), the dynamic right-hand side, an enum, `as ?Core\Class` (`E0473`, which refuses the nullable
spelling over *any* class target), and `new Core\X()`. `rule:core-classes/html-auto-escape` keeps
`"lit" as Core\Html\Markup` a lift: a lift's operand is a `Ty::Str`, so it never reaches the downcast
arm, which wants a `Ty::Tagged`. `crates/nvs-runtime/src/graph.rs`'s known gap is closed and its
`carried-gaps.md` row deleted.

## Next group

**Follow-on, unscheduled — one file set: `crates/nvs-ir/src/lower/closure.rs`,
`crates/nvs-types/src/expr/members.rs`.**

- [ ] **A closure parameter declared `Core\X` now class-checks, and nothing pins it** —
      `crates/nvs-ir/src/lower/closure.rs:404`'s `check_param_class` takes `declared_class`'s answer,
      which since this goal is `Some` for a `Core` class with instances, so a wrong argument raises a
      `LogicError` where it used to be checked for objecthood alone. `rule:types/class-reference-sites`
      is what specifies it; the entry check is reached from
      `crates/nvs-ir/src/lower/closure.rs:833`.
- [ ] **`new Core\X()` is refused for a reason no case states** —
      `crates/nvs-types/src/expr/members.rs:388`'s `undeclared_name` path is what a written `Core` class
      hits at a `new`, which reads as "undeclared" rather than as the deliberate refusal it is.
      `rule:types/class-reference-sites`.
- [ ] **The isolate round trip is asserted only through `mixed`** —
      `crates/nvs-stdlib/src/instance.rs:521`'s descriptors are what a decoded instance is rebuilt
      under, and no case names the class on the far side of a crossing.
      `rule:security/isolate-values-cross-by-copy`.

## Backlog

- `as ?Core\Class` is `E0473` like every class target, which `rule:types/conversion`'s general `as ?T`
  sentence does not mention — `docs/rules/types/conversion`.
- `Core\Html\Markup` is on `testable_core_class`'s roster and reaches `declared_class`; only the
  operand's representation keeps the lift and the downcast apart — `crates/nvs-ir/src/lower/closure.rs`.
