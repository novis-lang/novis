# Handoff

## State

**Goal `m4-refusals` — stage 7 is closed: `$f is callable(int): string` lowers and answers.** The
three halves that were open all landed this session. `lower_program` emits one field-less `ir::Class`
per `exprs.callable_sig_markers()` beside the marker every program gets, before the sort that keeps
the class list reproducible. Both synthesized closure classes carry `exprs.callable_markers_at(span)`
on `conforms` beside `CLOSURE_MARKER`. `test_shape` takes `exprs` and answers
`CheckedTy::CallableSig` with the marker's class row, which is the descriptor walk `callable` already
emits one step more specific.

- **`PendingClosure` gained the literal's span** (`crates/nvs-ir/src/lower/closure.rs:46`): the
  conformance table is keyed by the `fn` literal's own `expr.span`, and `FnExpr` carries none. The
  class label cannot be the key — it is this crate's name for the site and never reaches the checker.
- **The assignability directions are visible in the case**: a wider parameter (`fn(mixed): string`)
  and a shorter parameter list (`fn(): string`) both conform to `callable(int): string`, the return
  type alone is where the answer turns over, and a first-class callable answers by its target's
  signature. The relation is `nvs_types::expr::is_assignable`, read in `nvs_types::callables`.
- **`python tools/holes.py` is unchanged at 3 sites, `UNATTRIBUTED: 0`.** The `is` panic
  (`crates/nvs-ir/src/lower/expr.rs:4988`) keeps its `only lowers` wording and is now a backstop no
  checked program reaches — the two `None` sources left are named in `test_shape`'s `# Known gaps`.
- `python tools/verify.py` is 11 of 11 green over the whole group.
- Nothing is blocked.

## Next group

**Stage 8: declared types** — one file set: `crates/nvs-ir/src/lower/mod.rs` and
`crates/nvs-ir/src/lower/tests.rs`. This is the driver's earliest red check,
`docs/agent/loop-goal.toml:9698`, whose two named tests are the probes below.
`rule:types/grammar` owns the atom set; the goal prose's stage 8 owns what each row may answer.

- [ ] **The atoms `lower_decl_type` does not spell** — everything reaching the `other` panic
      (`crates/nvs-ir/src/lower/mod.rs:3150`) is `TypeKind::Intersection` plus the atoms `Null`,
      `Never`, `Iterable`, `Shape`, `Member` and the six tainted/secret `string`/`bytes` spellings;
      each takes the erasure `erase_checked_ty` gives its checked type
      (`crates/nvs-ir/src/lower/mod.rs:3386`). `Member` is the one no annotation answers from the AST
      alone, so it is the arm the `declared_ty` shortcut at
      `crates/nvs-ir/src/lower/mod.rs:3087` guarantees.
- [ ] **The two rows `erase_checked_ty` drops** — `CheckedTy::CoreShape` and `CheckedTy::TypeVar` at
      its `_ => return None` (`crates/nvs-ir/src/lower/mod.rs:3554`). Probe each from source before
      deciding, per the goal's stage 8: an options bag is flattened into per-slot constants and the
      checker substitutes every variable, so both may be engine invariants rather than erasures.
- [ ] **The two probes, and `lower_checked_ty`'s panic** — write
      `lower_decl_type_answers_every_type_atom_the_grammar_has` and
      `erase_checked_ty_answers_every_checked_type_a_value_can_have` in
      `crates/nvs-ir/src/lower/tests.rs`, counted over the atom list rather than read off a line
      (`conventions.md` § *A `.nvst` test case*'s invariance-over-a-sweep shape reads the same for a
      unit test). Once erasure cannot fail, the `unwrap_or_else` at
      `crates/nvs-ir/src/lower/mod.rs:3368` goes with it, closing the second of the three refusal
      sites `python tools/holes.py --sites` lists.

## Backlog

- Stage 9 needs the third site closed too: `test_shape`'s `None` and the panic at
  `crates/nvs-ir/src/lower/expr.rs:4988`. Its `# Known gaps` names the only two shapes that still
  answer `None`, and both are guaranteed unreachable rather than unwritten.
- `crates/nvs-types/src/callables.rs` is the design home for the marker device; nothing there needs
  re-deciding, and `nvs-ir` now reads every field the table exposes.
