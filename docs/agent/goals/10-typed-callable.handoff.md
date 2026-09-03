# Handoff

## State

**Goal 10 — a `callable` carries its signature — has just started; nothing of it has landed yet.** Goal
9's whole list is this goal's Stage 1 floor. The design is settled and is not to be re-derived:
[ADR 0136](../adr/0136-a-callable-carries-its-signature.md) landed with this goal and its five decisions
were taken with the user — the spelling `callable(T, U): R` with a mandatory return and no parameter
names, bare `callable` kept as the top of the lattice, arity a prefix match (`n ≤ m`, matching what
`call_closure` already does when it trims), parameters contravariant with a covariant return, and a `fn`
literal taking its parameter types from the position it is written in. The goal prose's standing
decisions carry the rest, including the `E08xx` band the new diagnostics open at `E0800`.

The reason this goal is worth its sessions is `crates/nvs-runtime/src/closure.rs`'s own module doc:
`check_param_tags` is a **priority 1** guard standing between a mismatched argument and an arbitrary
dereference, paid per argument per call. This goal makes it a compile-time proof.

## Next group

**Stage 2: the atom and what it compares to** — one file set:
`crates/nvs-syntax/src/parser/ty.rs`, `crates/nvs-syntax/src/ast.rs`, `crates/nvs-types/src/ty.rs`,
`crates/nvs-types/src/expr/assign.rs`.

- [ ] **The production** — `parse_type` (`crates/nvs-syntax/src/parser/ty.rs:167`) parses
      ADR 0136 § 1's `'callable' '(' (type (',' type)*)? ')' ':' type`. A `(` after `callable` is
      unambiguous in type position, so no checkpointed trial parse is needed. A missing `: R` and a named
      parameter are diagnostics here — the first two codes in the new `E08xx` band, declared with the
      band's legend row in `crates/nvs-diagnostics/src/lib.rs`.
- [ ] **The representation** — a `Ty::CallableSig { params, ret }` beside `Ty::Callable`
      (`crates/nvs-types/src/ty.rs:170`), interned like every other type, with the display arm at `:520`
      rendering it as written. Leave `Ty::CallableTo` (`:188`) and `Ty::CallableShapeTo` (`:208`) alone —
      they are retired in stages 4 and 5, not here.
- [ ] **Assignability** — `is_assignable` (`crates/nvs-types/src/expr/assign.rs:56`) gains ADR 0136
      §§ 3-4. The seven named tests of the TOML's stage 2 `nvs-types` check are the shape of it; § 4 of
      the ADR is the one home for why `array<T>`'s invariance does not reach this relation.

## Backlog

- Stage 3 (inference at the `fn` literal — `crates/nvs-types/src/expr/calls.rs:1513`,
  `crates/nvs-types/src/expr/args.rs:1129`) shares only the `nvs-types` crate with stage 2, not its
  files. A session that lands stage 2 with headroom under 120k should take it anyway: the checker is
  already loaded and the expected-type plumbing is what stage 2's assignability rule exists to feed.
- Stages 4 and 5 retire `CoreTy::CallableTo` and `CoreTy::CallableShapeTo` respectively, each with its
  spec edit in the *same* slice as the registry rows it must agree with.
- Stage 6 is the codegen and the valgrind leg, and it is the only stage that touches `nvs-ir`,
  `nvs-codegen` and `nvs-runtime`. It shares nothing with the four before it — expect it to want its own
  session.
- When this goal's last check goes green the chain is finished. The milestone table's order 6 is M4B,
  whose staged goal is `docs/agent/next-goal-m4b.md`.
