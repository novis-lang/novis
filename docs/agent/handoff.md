# Handoff

## State

**Goal `type-test` — stages 0 through 5 are on disk, and a program can run an `is`.** The operator
parses, answers `bool`, folds a settled answer, refuses only its right-hand side, narrows the true
edge, and now lowers: `crates/nvs-ir/src/lower/expr.rs`'s `lower_type_test` is the whole of it and
`crates/nvs-types/src/expr/type_test.rs` is the checker half.

**The recorded entry is the contract, and it is now two variants.** `ExprInfo::TypeTest { tested }`
means a run-time `bool` and carries the interned right-hand side; `ExprInfo::SettledTypeTest
{ answer }` means the checker settled it and carries the constant. Narrowing keys on the first
variant, so its guard is unchanged. The second exists because the fold is **not** re-derivable from
the erasures `nvs-ir` holds — the playbook bullet added this session is that trap.

**Three shapes lower, and none of them is a second walk.** A scalar, `null`, `object` and a bare
`array` are one masked tag compare (`InstKind::TagIs`, the low byte alone because a `decimal` spells
its scale and sign in the rest of the word); a class or interface is the `InstKind::InstanceOf`
`instanceof` and `as C` already emit; an `array<T>` is `Helper::ToArrayOfOrNull`, which is
`as ?array<T>`'s spelling of the walk `as array<T>` throws from. A subject that is not `Ty::Tagged`
carries one known tag, so its answer is a constant.

**What does not lower yet is the payload rows.** A literal, an enum case, a shape, `iterable`,
`callable` and an `array<T>` whose element type no tag decides all reach `lower_type_test`'s panic.
`examples/type-test.nvs` (stage 6) wants `enum-case=1` and `literal=1`, so the fixture cannot exist
until the first two land — the driver's red check naming that file is an item still open.

## Next group

**Stage 5 finished, then stage 6's fixture** — one file set: `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-codegen/tests/objects.rs`, `examples/type-test.nvs`.

- [ ] **A literal type is the tag compare plus a payload compare** — `TestShape` at
      `crates/nvs-ir/src/lower/expr.rs:5040` returns `None` for `CheckedTy::IntLiteral`,
      `StringLiteral`, `True` and `False` today, and `lower_type_test` at
      `crates/nvs-ir/src/lower/expr.rs:4758` panics on it. The tag is not the whole test here, so
      the two comparisons are `&&`-shaped: a `string` literal's payload compare is a call and must
      not run on a value whose tag is not `Tag::Str`. `rule:types/type-test`.
- [ ] **An enum case is the same shape over an integer** — `CheckedTy::EnumCase(_, backing, _)`
      at `crates/nvs-ir/src/lower/expr.rs:5040`; the case's own value is the checker's
      (`rule:enums/representation` makes a case its backing integer), and the tag is that backing
      type's, so this is one payload compare with no call in it. `rule:types/enum-case-type`.
- [ ] **`examples/type-test.nvs`, the thirteen lines the acceptance check pins** — the `want` list at
      `docs/agent/loop-goal.toml:7185` is the specification, in order, and every row of it has a
      lowering once the two items above land. `rule:types/type-test`.

## Backlog

- **`CEILING` in `crates/nvs-ir/tests/refusals.rs` rose to 16 for `lower_type_test`'s panic** and comes
  back to 15 in the slice that lowers the last row. Its doc comment says so; nothing else may raise it.
- `is array<Foo>` panics in lowering: no tag word exists for a class element, and `as array<Foo>` is
  `E0711` where `is` refuses nothing — `crates/nvs-ir/src/lower/expr.rs`'s `test_shape` § *Known gaps*.
- `is iterable` and `is callable` have no lowering row at all — same function, same panic.
- A `mixed` holding an enum answers `is int` as its backing tag would, because `tag_of` gives an enum
  the backing type's tag — `crates/nvs-codegen/src/ty.rs` owns that decision and its own comment.
- The reference heading and precedence row for `is` (`python tools/reference.py --check`), stage 6.
- The conformance cases for `is`, stage 6's `nvs-suite` check.
