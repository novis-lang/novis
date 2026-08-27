# Handoff

## State

**M4 — language completeness.** `as ?T` over a literal type, an enum-case subset or a
whole enum runs: ADR 0066 § 3 row 2's "non-throwing twin", built as a twin of the
checked chain rather than as a redirect of its error edge. `mixed`, `int`, `string` and
`bool` operands all reach it, and the two spellings agree row for row — the nullable one
answers `null` on exactly the values the checked one throws on, and coerces no more than
it does. `python tools/holes.py` reads **24 sites, 6 items**, unchanged: `convert_or_null`'s
catch-all still stands for `as ?string` and `as ?array<T>`, which this slice did not widen.

`verify.py` 6 of 6 green — conformance **603**, differential **167**, 1632 unit tests.
`tools/leak-check.sh` green over two fixtures covering the new refcount edges: a fresh
`Ty::Str` operand tagged into the answer and released on the miss edge, a borrowed `mixed`
retained on the hit edge, and a fallible base conversion whose `null` reaches the same miss.

Facts recorded where they belong rather than here: `mwl_ir::lower::expr`'s
`Lowering::lower_nullable_membership` owns why this is a twin and not a redirect, and why a
fallible base compares tagged; `Lowering::nullable_target_atoms` owns why the target is read
off the whole annotation; `Lowering::closed_set_of_atoms` owns why `operand_names_one_value`
does not apply to a nullable target.

## Next group

**The two remaining panics in `crates/mwl-ir/src/lower/expr.rs`'s own dispatch**, still the
file this session had open, with `crates/mwl-ir/src/lower/mod.rs` (`lower_decl_type:2266`,
`erase_checked_ty`) beside it. Item [3] below is the odd one out and is listed last on
purpose: it is the only one needing a `mwl_types` half first.

- [ ] **The `lower_expr` dispatch catch-all** — `crates/mwl-ir/src/lower/expr.rs:259`. Its
      message lists what *is* lowered, so the first job is the complement: enumerate the
      `ExprKind` variants that still reach it (`crates/mwl-syntax/src/ast.rs`'s `ExprKind`),
      and split them into the ones with a lowering to write and the ones the language does
      not have, which take a diagnostic naming the rule instead. `Class::method(...)` is
      already known to be there (`mwl-ir` gap 1, playbook) and calling a closure through the
      variable holding it is the other named one. **Do not write a lowering for all of them
      in one slice** — write the enumeration into the handoff and take the cheapest half.
- [ ] **`$b as ?string`, the last shape `convert_or_null` refuses** —
      `crates/mwl-ir/src/lower/expr.rs:982`. ADR 0066 § 3 row 1 makes it available (`bytes as
      string` is a row ADR 0007 § 2 defines and it can fail), and the panic's own text calls
      it "cannot fail", which is true of `int as ?string` and not of this one. One new
      `Helper::BytesToStringOrNull` beside `Helper::BytesToString`
      (`crates/mwl-ir/src/ir.rs:1437`, `crates/mwl-runtime/src/helpers.rs:769` for the shape,
      `crates/mwl-codegen/src/emit.rs:3141` for the name map) and one row in
      `convert_or_null:970`. An **object** target is the separate half and is not this one:
      `as string` there is a resolved `toString` call, so a `?` form needs the call's own
      error edge redirected, which is the design this session rejected for the set chain.
- [ ] **A `Class::CONST` on a user-declared class** —
      `crates/mwl-ir/src/lower/expr.rs:248`. The panic is honest: `mwl_types` models no value
      for one, so the checker half lands first (the goal's standing decision on ordering), and
      `ExprInfo::CoreConst`'s emitter (`emit_const_arg`) is what the lowering half then reuses.

## Backlog

- An enum case tagged into a `mixed` reads as its backing integer, so it is falsy where ADR
  0035 § 4 makes every statically-typed case truthy — `mwl_codegen::ty::tag_of:93`, whose
  comment's premise expired when `mixed` became `Ty::Tagged`. `Core\Reflect::typeOf` over the
  same value is the same gap by a second route.
- `Core\Json::decodeAs<T>`'s wider codec-reachable set and its two default-bearing rows —
  `mwl_stdlib::json`'s own module doc.
- ADR 0088's qualifier classification — `mwl_stdlib::hash`'s module doc.
- ADR 0086 § 1's substitution table — `crates/mwl-stdlib/src/cli.rs` gap 1, M8.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
