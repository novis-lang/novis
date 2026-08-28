# Handoff

## State

**M4, Stage 00, and three of its six shapes are closed.** Item 45 landed this session: a
user-declared class constant now has a declared type and a value.
`nvs_types::signatures::ConstSig` holds both — the written annotation interned in the pass that
already holds the declaration and the value together, and the value placed in that type by
`defaults::literal_default`, so `const uint WIDE = 5;` arrives as a `ConstUint`. A read resolves
through `extends`, `implements` and `self::` (`signatures::resolve_const`), records the value as
`ExprInfo::CoreConst` and lowers to exactly the instruction the written literal does. ADR 0109's
`for` header and item 50's `E0239` are unchanged from the previous session.

`crates/nvs-types/src/consts.rs` keeps its own job and is not a duplicate: it folds a value for
ADR 0047 § 2's *type* position and for ADR 0033 § 4's `secret` bit, and it runs before there is an
interner, which is the whole reason the type lives one table over. Seven doc sites that said a
class constant's type was unmodeled are reconciled, ADR 0033's own sentence included — the sink
still reads the bit from `ConstTable` because a payload is folded before the class name in it is
resolved, which is now the stated reason.

**One shape is left unlowered and it is narrower than the item**: a constant whose value has no
constant form at all (`const ROWS = [1, 2];`) reads as `mixed` and still panics
`nvs_ir::lower::expr`'s `ClassConstAccess` arm, whose message now names that shape alone. The arm
count `refusals.rs` ceilings is unchanged.

**Item 46 has a design question inside it, found while scoping and not yet answered.** `static` in
a return annotation interns to the *declaring* class in `lower.rs:143`'s `resolve_special`, so a
call site cannot tell it from `self`. Recording a `returns_static` bit on `MethodSig` and
substituting the called class at `calls.rs:156`/`calls.rs:268` is the contained fix — but it is
unsound as stated: `Base::make(): static { return new self(); }` type-checks inside the body
(where `static` *is* `Base`) and would then hand a `Leaf`-typed binding a `Base`. PHP catches that
at run time; Novis has no such check. Decide it in the session that lands it — refusing the body,
or accepting the hole and pinning it — and record the decision where the bit is declared.

## Next group

**Items 46-47, the rest of the gate's `nvs-types` check.** One file set:
`crates/nvs-types/src/expr/calls.rs`, `crates/nvs-types/src/signatures.rs`,
`crates/nvs-types/src/locals.rs`, with tests in `crates/nvs-types/tests/`.

- [ ] **Item 46 — `new static()` and a `: static` return resolve to the called class.**
      ADR 0008's late static binding, as a type. `crates/nvs-types/src/lower.rs:143` is where
      `static` loses its identity; `crates/nvs-types/src/signatures.rs:53` is `MethodSig` and
      `:883` its one user-source construction site (four more: `core_lib.rs:89`,
      `error_lib.rs:189`, `iter_lib.rs:153`, `expr/args.rs:934`); `crates/nvs-types/src/expr/calls.rs:156`
      and `:268` are the two `return_ty` sites to substitute at. `ResolvedCall::static_class`
      already carries the called class for *dispatch* — read it before inventing a second one.
      Gate name: `a_static_return_type_resolves_to_the_called_class`.
- [ ] **Item 47 — `instanceof` narrows to an interface, not only to a class.**
      `crates/nvs-types/src/locals.rs` owns the residue; the class direction is green
      (`crates/nvs-types/tests/narrowing.rs:145`). Gate name:
      `an_instanceof_test_narrows_its_subject_to_an_interface` — and note
      `docs/agent/guard-name-debt.md` § *Stage 00's remaining names* on why the near-twin already
      in the tree must not be renamed into it.
- [ ] **Item 48 — `new` on a `Core` class with no constructor is a diagnostic.** Same crate, the
      `new` half of `calls.rs`. Gate name: `a_core_class_with_no_constructor_refuses_arguments`.

## Backlog

- A class constant whose value has no constant form (`const ROWS = [1, 2];`) panics `nvs-ir`
  rather than being diagnosed — `crates/nvs-types/src/lib.rs`'s known gaps.
- ADR 0046 § 5's payload folder could now resolve a class constant, but `retrieval.rs`'s
  `fold_value` carries no namespace context to resolve the class name with — that module's docs.
- Item 49's `every_spellable_expression_reaches_a_diagnostic_or_an_ir`, the shape table that
  would have found item 45 — `docs/agent/loop-goal.md`.
- `refusals.rs`' recognizer matches a refusal by phrasing, so its ceiling of 4 is a floor —
  item 49 owns it.
