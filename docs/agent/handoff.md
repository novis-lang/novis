# Handoff

## State

**Goal 18, stage 5 items 1 and 2 are landed.** `examples/input-shapes.nvs` and the
`examples/input-shapes.nvsr` fixture beside it run under `nvs run --request` and print exactly the
six lines `docs/agent/loop-goal.toml:5810`'s `want` names, in that order, so the acceptance check
that has been failing since the goal opened closes on the driver's next run. `docs/reference/core/Arr.md`
carries `Core\Arr::shapeAs`'s paragraph and one worked example, and `python tools/reference.py`
reports 291 of 291 examples holding.

**Both request call sites in the example are written `tainted {…}`, not an unqualified shape.** That
compiles today and is what stage 2's owed diagnostic will require, so the file does not need
rewriting when that lands.

**Writing the example uncovered a defect in stage 1's hydration, now fixed.** An absent optional
field on a hydrated shape threw under `??` and `isset` alike, against `rule:types/shape-type`'s last
paragraph: the slot carries `Tag::Unset` and the erased reader refused it ahead of the guard. A
guarded read now answers `null` when `ClassDesc::is_shape` holds, and a `lateinit` slot still throws.
The playbook's *Writing Novis itself* owns why one arm serves both.

**Stage 5's third item is not a test-writing slice.** `Core\Request::postAs<{email: string}>()`
compiles today: `E_DECODED_FIELD_NOT_TAINTED` walks derive classes only, never an inline shape
written as a type argument, so the checker edit comes before the reject case can be written.

## Next group

**Stage 5: the diagnostic corpus, checker first** — one file set:
`crates/nvs-types/src/derive.rs`, `crates/nvs-types/src/expr/`, `tests/conformance/reject/`.
loop-goal.md § *Stage 5* is the specification, and its first two clauses need the checker before
they need a case.

- [ ] **The unqualified-shape check at a tainted call site** — `rule:security/tainted-qualifier` and
      the goal's § *Standing decisions* ("the taint diagnosis stays on"). `check_decode_sites` at
      `crates/nvs-types/src/derive.rs:821` is the whole of the existing walk and it iterates
      `CodecFieldSite` rows for a derive class, so a `Ty::Shape` written as a type argument reaches
      it never. The call site's shape label is recorded at `crates/nvs-types/src/expr/args.rs:1451`.
- [ ] **Its `--EXPECTF-ERROR--` case**, once the diagnostic exists — the section shape, whose
      indentation widens with the line number, is
      `tests/conformance/reject/a-core-io-path-refuses-a-tainted-argument.nvst:28`.
- [ ] **`a shape whose tainted promises nothing is refused`** — this one owes only its case: the
      diagnostic is already emitted at `crates/nvs-syntax/src/parser/ty.rs:354`, so `tainted {n: int}`
      is refused today and the case pins what it says.
- [ ] **`a missing required key names the key`** — the shape-literal half, refused by
      assignability rather than at run time. `is_assignable` is `crates/nvs-types/src/expr/assign.rs:60`
      and `rule:types/shape-type` is what it implements.

## Backlog

- `docs/rules/classes/an-unwritten-property-read-throws.md:7` still says two declarations reach the
  unwritten state; a shape hydration is a third. Whether a factual correction needs a record is the
  open question — the goal's § *Standing decisions* enumerates the rules it may change and this is
  not one of them.
- `docs/reference/core/Arr.md`'s front-matter `keywords` names PHP functions only and carries no term
  for the shape conversion — `docs/agent/doc-style.md`.
- Spec §§ 6 and 15 still owe `Core\Arr::shapeAs` a roster row; `crates/nvs-stdlib/src/arr.rs:703`'s
  comment says so and goes when it lands.
