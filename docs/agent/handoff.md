# Handoff

## State

**Goal `m4-refusals` — stage 8 is closed: every type atom lowers as a declared type, and every
checked type a value can have erases.** `lower_decl_type` spells all 34 `TypeAtom` variants and all
5 `TypeKind` ones; `erase_checked_ty` spells all 37 `CheckedTy` rows and returns `Ty` rather than
`Option<Ty>`, so `lower_checked_ty` is gone — its name was the one 38 sites used, and the total
function took the name the goal's own check names. `python tools/holes.py` is down to **1 site**,
`crates/nvs-ir/src/lower/expr.rs:4986`, and `CEILING` fell to 1 with it.

- **A type variable does reach erasure from a program, and `Ty::Tagged` is its answer.**
  `rule:classes/delegation-by-field`'s forward is written against the *interface's* signature, and a
  compiler-owned generic's signature still names `T` (`crates/nvs-ir/src/lower/call.rs:1285`). Before
  this, `Iterator<int> by $field` synthesized no forward at all and the program died with
  `a method with no body was called` — the new conformance case is that program.
- **`CheckedTy::CoreShape` is an engine invariant, probed rather than assumed**: a `Core` options bag
  is one argument per merged slot, flattened at `crates/nvs-ir/src/lower/call.rs:203` before
  `ArgSig::expectation` erases anything. Both case trees stay green with that arm asserting.
- `python tools/verify.py` is 11 of 11 green over the whole group; conformance is 1893.
- Nothing is blocked.

## Next group

**Stage 9: the gate** — one file set: `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-ir/tests/refusals.rs`, `crates/nvs-ir/src/lib.rs` and `docs/agent/carried-refusals.md`.
The goal prose's stage 9 owns all three bullets; `rule:types/type-test` owns what `is` answers.

- [ ] **The `is` backstop is the last refusal site** — the panic at
      `crates/nvs-ir/src/lower/expr.rs:4986` still says `only lowers`, which `tools/holes.py`'s
      `REFUSAL` reads as a hole, but stage 7 closed the shape and `test_shape`'s `# Known gaps` names
      the only two `None` sources, both guaranteed unreachable. Rewrite it as the engine invariant it
      now is, naming what guarantees it — the playbook bullet on `#[non_exhaustive]` is the same move.
- [ ] **`CEILING` to 0, its doc comment rewritten whole** — `crates/nvs-ir/tests/refusals.rs:94`,
      saying what `0` means: a new refusal site is a red test, full stop.
- [ ] **The gaps and the entry that claimed the closed sites** — `crates/nvs-ir/src/lib.rs:185`
      § *Known gaps* (gaps 1 at `:209` and 6 at `:315` keep only their other halves), and entry 901 at
      `docs/agent/carried-refusals.md:25`, which retires with the playbook bullet through their shared
      `[until: exists crates/nvs-ir/tests/refusals.rs:const CEILING: usize = 0;]` — `python
      tools/playbook.py --retire` or the wrap, never by hand.

## Backlog

- The checker half of the generic-interface forward: `crates/nvs-types/src/conformance.rs:437` records
  the interface's *unsubstituted* params, so `Iterator<int>::current()` forwards through `Ty::Tagged`
  where `int` would do. Correct, not free; a substitution there would keep the slot typed.
- `docs/plan/m4.md`'s stale 1000-case figure and `done*` belong to goal `plan-truth`.
- `crates/nvs-types/src/ty.rs:198` carries two doc paragraphs on one variant — the `callable`-plus-
  type-variable sentence describes no row `ShapeOfCallables` has.
