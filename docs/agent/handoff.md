# Handoff

## State

**Goal 10 — a `callable` carries its signature. Stage 2's first slice is on disk: the production
parses.** `callable(T, U): R` is `TypeAtom::CallableSig { params, ret }`
(`crates/nvs-syntax/src/ast.rs:220`), built by `Parser::parse_callable_type`
(`crates/nvs-syntax/src/parser/ty.rs:614`) off one token of lookahead — a `(` after `callable`, no
checkpointed trial parse. The two refusals `rule:types/callable-signature` names are `E0800`
(a named parameter, reported once per type) and `E0807` (no return type); both recover as **bare
`callable`**, so a misspelt signature cascades nowhere.

**Nothing checks a signature yet.** `crates/nvs-types/src/lower.rs:149` lowers the new atom to the
lattice top, so the annotation today admits exactly what bare `callable` admits and a call through it
keeps the dynamic path — the guard `crates/nvs-runtime/src/closure.rs`'s module doc calls priority 1
is still paid per argument. `crates/nvs-ir/src/lower/mod.rs` erases it to `Ty::Object` beside
`Callable`. The three walks that read names out of a type expression — `nvs_hir::requires::walk_type`,
`nvs_hir::aliases::record_names` and `::substitute` — learned the atom, so a class named only inside a
signature still pulls its file in and an alias inside one still expands.

The design is settled and is not re-derived: `rule:types/callable-signature` plus the goal's own
§ *Standing decisions*.

## Next group

**Stage 2 slices 2 and 3: the representation and what it compares to** — one file set:
`crates/nvs-types/src/ty.rs`, `crates/nvs-types/src/lower.rs`, `crates/nvs-types/src/expr/assign.rs`.
Take both: slice 2 alone leaves the new atom representable and assignable from nothing, which is a
worse resting state than the lattice-top interim on disk now.

- [ ] **The representation** — a `Ty::CallableSig { params: Vec<TypeId>, ret: TypeId }` beside
      `Ty::Callable` (`crates/nvs-types/src/ty.rs:171`), an interner constructor beside
      `TypeInterner::callable` (`crates/nvs-types/src/ty.rs:775`), and a `describe` arm beside the one
      that renders `Ty::Callable` (`crates/nvs-types/src/ty.rs:537`). Then
      `crates/nvs-types/src/lower.rs:149` builds it instead of answering the lattice top.
      `rule:types/callable-signature`. `Ty::CallableTo` (`crates/nvs-types/src/ty.rs:189`) and
      `Ty::CallableShapeTo` (`crates/nvs-types/src/ty.rs:209`) are what ADR 0136 § *In short* retires
      once this exists; deleting them is stage 4's, not this slice's.
- [ ] **Assignability** — `is_assignable` (`crates/nvs-types/src/expr/assign.rs:56`) gains
      `rule:types/callable-arity`'s prefix match (`n ≤ m`, first `n` parameters compared) and
      `rule:types/callable-variance` (parameters contravariant, return covariant). Every
      `CallableSig` is assignable to bare `Ty::Callable`, which is the lattice top and stays so.

## Backlog

- Stage 3: a `fn` literal takes its parameter types from the expected position —
  `rule:types/callable-literal-inference`.
- Stage 4: retire `CoreTy::CallableTo`/`CallableShapeTo` from `crates/nvs-stdlib/src/registry.rs`,
  editing `docs/spec/01-core-library.md` in the same slice — `rule:core-api/reference-card`.
- Stage 5: `Core\Task::all`'s "must be a written `fn` literal" restriction lifts —
  `rule:concurrency/an-all-field-must-be-a-written-fn-literal`.
- Not to be attempted here: `E0450` is not relaxed and whole-body return inference is not tried —
  the goal's § *Standing decisions*.
- `nvs_hir::aliases::record_names`/`substitute` still skip `TypeAtom::Shape`, so an alias inside a
  shape type does not expand. Pre-existing, not this goal's; `crates/nvs-hir/src/aliases.rs:194`.
