# Handoff

## State

**Goal 10 — stage 2 is whole: the signature parses, interns, renders and compares.**
`Ty::CallableSig { params, ret }` (`crates/nvs-types/src/ty.rs:172`) interns like any other type,
`TypeInterner::callable_sig` builds it, `describe` renders the source spelling back, and
`crates/nvs-types/src/lower.rs:149` lowers the atom field-wise. `is_assignable`
(`crates/nvs-types/src/expr/assign.rs:60`) carries the lattice: every signature satisfies bare
`callable` and nothing satisfies a signature from above, two signatures compare by
`rule:types/callable-arity`'s prefix match under `rule:types/callable-variance`, and `never` is the
bottom of the return position. Four wildcard sites learned the variant — the isolate entry refusal,
`wants_callable`, `generics`' two walks, and `nvs_ir::lower::erase_checked_ty` (to `Ty::Object`,
beside bare `callable`).

**The interim state refuses every program that writes the annotation, and stage 3 is what closes
it.** A `fn` literal's own type is still bare `Ty::Callable`, so
`callable(int): string $r = fn (int $n): string => "n";` is an `E0401` reading *expected
`callable(int): string`, found `callable`* — the rendering is right and the relation is right; what
is missing is a literal that carries its signature. Nothing in the tree writes the annotation yet,
so no test, example or fixture regressed.

The design is settled and is not re-derived: `rule:types/callable-signature`, `-arity`, `-variance`,
plus the goal's own § *Standing decisions*.

## Next group

**Stage 3: the inference that makes it free** — one file set: `crates/nvs-types/src/expr/calls.rs`,
`crates/nvs-types/src/expr/args.rs`. Take both slices: the first alone types an annotated literal
and leaves `Core\Arr::map($users, fn($u) => $u->name)` exactly where it is.

- [ ] **A `fn` literal carries its own signature.** `check_fn_literal`
      (`crates/nvs-types/src/expr/calls.rs:1513`) answers `Ty::CallableSig` built from the
      parameters' declared types and the declared return type, instead of the bare
      `Ty::Callable` it answers now — which is what makes the State block's `E0401` go away, since
      `is_assignable` already has every row it needs. `rule:types/callable-literal-inference`.
- [ ] **An unannotated parameter takes its type from the position the literal is written in.**
      `check_fn_literal` gains an expected type; each unannotated parameter takes the corresponding
      one from it, and an annotated parameter is checked against it under
      `rule:types/callable-variance` and wins where it is wider. The argument position that hands it
      over is `crates/nvs-types/src/expr/args.rs:540`. `E0450` is not relaxed —
      `rule:types/callable-literal-inference` and the goal's § *Standing decisions* both say so.

## Backlog

- Stage 4: `CoreTy::CallableTo` deleted, five registry rows write an ordinary type, and
  `docs/spec/01-core-library.md` § *Arr* edited in the same slice — `docs/agent/loop-goal.md`.
- Stage 5: `CoreTy::CallableShapeTo` deleted and `Task::all`'s written-literal restriction removed,
  in the checker and the spec together — `docs/agent/loop-goal.md`.
- Stage 6: a proven call site emits no per-argument tag check; the valgrind leg is not optional
  there — `docs/agent/loop-goal.md`.
- `examples/typed-callable.nvs`, the acceptance fixture the driver is red on, is writable the moment
  stage 3's first slice lands — `docs/agent/loop-goal.toml`.
- `[context] rules` named ADR 0136 and printed its four rules by **title only**; the two this group
  implemented had to be peeked. Naming `types/callable-arity` and `types/callable-variance` as rule
  tokens in that field inlines the fragments instead.
