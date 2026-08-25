# Handoff

## State

**Stage 0 items 1-6 and 9 are done; item 7 is one third done and item 8 is untouched.**
`mwl_types::iter_lib` now seeds *all four* compiler-declared global interfaces rather than only the two
generic ones ([iter_lib.rs:69](../../crates/mwl-types/src/iter_lib.rs#L69)): `Comparable` declares
`compareTo(Comparable $other): int` and `Stringable` declares `toString(): string`, both bodiless and
`Visibility::Public`. `mwl_hir::interfaces` gained `COMPARABLE`/`STRINGABLE` beside `ITERABLE`/`ITERATOR`
so neither half spells a name the other owns. ADR 0013 § 1 writes the parameter as `self`; the seeded
declaration *is* `Comparable`, and that module's own docs say why the wider spelling costs nothing (the
same-class rule is the operator's, in `expr::operators::object_comparison_result`). `compareTo` returns
`int`, settled against `mwl_stdlib::ordering`, which is `std::cmp::Ordering` and not an MWL-visible type.

Consequence, and it is the point: `crate::conformance` reads that table, so `class Money implements
Comparable {}` is now `E0449` like any other unanswered interface. Two `mwl-types` integration fixtures
declared exactly that and were given bodies.

`python tools/verify.py` is green (1369 tests), `mwl test tests/` is 429/0, and every `examples/*.mwl`
runs as before (`collect.mwl` still fails on unbuilt `Core` members, `uncaught.mwl` still exits 1 by
design).

## Next group — Stage 0 item 7's two remaining thirds, then item 8

**Shared file set:** `crates/mwl-types/src/expr/members.rs`, `crates/mwl-ir/src/lower/expr.rs`,
`crates/mwl-codegen/src/lib.rs` and `tests/conformance/class/`. The rule is `docs/agent/loop-goal.md`
§ *Stage 0* items 7 and 8.

- [ ] **7b — `instanceof Stringable` must record a resolved class, and something must exist to test
      against.** The checker half is one condition:
      [members.rs:161](../../crates/mwl-types/src/expr/members.rs#L161) records an `ExprInfo::InstanceOf`
      only for a declared symbol or a reserved global *class*, so add the interface roster
      (`QName::is_reserved_global_interface`). The runtime half is the real work and is not a panic —
      `emit_instanceof` ([emit.rs:1399](../../crates/mwl-codegen/src/emit.rs#L1399)) returns
      `CodegenError::Unsupported` because `Classes::build`
      ([lib.rs:433](../../crates/mwl-codegen/src/lib.rs#L433)) builds descriptors from
      `mwl_ir::ir::Class` and no source declares `Stringable`. `mwl_runtime::object::is_instance_of`
      ([object.rs:768](../../crates/mwl-runtime/src/object.rs#L768)) already answers an *interface*
      ancestor, so what is missing is a `ClassDesc` for the reserved four and an ancestor edge from each
      implementor. `lower_instanceof` is [expr.rs:2790](../../crates/mwl-ir/src/lower/expr.rs#L2790).
- [ ] **7c — `echo $s` on a value typed at the interface itself.** `require_stringable`
      ([operators.rs:792](../../crates/mwl-types/src/expr/operators.rs#L792)) asks
      `mwl_hir::implements_interface` ([hierarchy.rs:367](../../crates/mwl-hir/src/hierarchy.rs#L367)),
      which walks *parents* only and so answers `false` for `Stringable` against `Stringable` — so
      `$s->toString()` checks but `echo $s` is `E0412`. Decide it at the call site or in the walk, then
      write the `.mwlt` cases item 7 asks for under `tests/conformance/class/`: a `Stringable` parameter
      whose `toString()` is called and echoed, and a `Comparable` one ordered through `<`. Both are
      recorded in `mwl-types`' known gaps ([lib.rs:205](../../crates/mwl-types/src/lib.rs#L205)).
- [ ] **8 — ADR 0061's `autoload`**, both file-scope declaration forms and the name-to-file fixpoint over
      the require-graph worklist. `loop-goal.toml` already waits on `an_autoload_declaration_parses`
      (`mwl-syntax`) and `an_autoload_declaration_resolves_a_name_to_its_file` (`mwl-hir`). Different file
      set from 7b/7c — start a session on it rather than tacking it onto one.

## Backlog

- ADR 0094's one uncovered shape: a promoted constructor parameter is no table's property, so nothing
  resolves it to check — `mwl-types`' `signatures` known gaps.
- `private(set)` is ADR 0094 § 3's write half and is not modeled at all — same known-gap list.
- ADR 0043 § 4's `by $field` delegation exempts a whole class from conformance — `mwl-types`'
  `conformance` module doc.
- No override-compatibility check exists, so ADR 0013 § 1's `compareTo(self)` variance rule is unenforced
  — `mwl-types`' `signatures`.
- `mwl-ir` gap 19: `$n + $f` and `$n < $f` type-check and still fail in codegen.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.

`orient.py` printed everything this session needed.
