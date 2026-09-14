# Handoff

## State

**Goal `m4-refusals` — Stage 7's element row is landed.** `$x is array<Foo>` lowers and walks every
element, and every other row composes inside it: a subclass, an interface, a shape, a union, a
nullable element, and an array nested one level down. `python tools/holes.py` reports **3** refusal
sites, `UNATTRIBUTED: 0`, **15** guarded, and `crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **3**
to match — unchanged, because the one panic that carried this gap still stands for the row left
under it.

- **The walk is lowered IR, not a runtime helper.** `TestShape::Every`
  (`crates/nvs-ir/src/lower/expr.rs:5797`) is a loop over the cursor `foreach` steps
  (`InstKind::ArrayNextSlot`/`ArrayValueAt`), the element read at `Ty::Tagged` and handed to its own
  row. A tag word has no room for a class label and no helper can be handed a `TestShape` across
  the ABI, so widening `array_element_tags` was not an option; there is no `as array<Foo>` to
  share, that spelling being refused where it is written (`E0711`).
- **`TestShape::ArrayOf`'s tag word stays for the element types it can say.** The helper walks the
  whole array inside the runtime while the lowered loop pays two calls and a branch per element, so
  the split is a latency one (priority 3) and not two answers to one question — the conformance
  case asserts the two agree.
- **Nothing is retained by the walk.** `ArrayValueAt` borrows, the element's row only reads, and the
  `Untag` of a `Ty::Tagged` subject is the same reference one representation down on the edge the
  tag compare proved. `wsl.exe -- bash /mnt/d/mwl/tools/leak-check.sh` over a fixture exercising it
  reported 0 failures.
- **The floor's link gate was red on a regression, not on unwritten work.**
  `tools/playbook.py`'s `DELIBERATE_STALE` is keyed by paths that are absent by construction, and
  `check-links.py` read its one key as a dead citation. Both existing markers would have been false
  of it, so there is now a third, `check-links:subject`.
- **Stage 7 is not finished.** A written `callable` signature is the one type `test_shape` still
  answers `None` for; `$f is callable(int): string` reaches
  `crates/nvs-ir/src/lower/expr.rs:4986`'s panic today.
- Nothing is blocked.

## Next group

**Stage 7: the written `callable` signature row** — one file set:
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-ir/src/lower/closure.rs` and
`tests/conformance/lang/an-is-test-against-iterable-or-callable-answers-by-what-the-value-holds.nvst`.
`rule:types/type-test` is the rule — its table puts a callable signature on the right and refuses
nothing else — and `rule:types/callable-signature` owns the type.

- [ ] **Decide what a closure carries at run time that names its signature** — `nvs_types::Ty`'s
      `CallableSig` (`crates/nvs-types/src/ty.rs:192`) says a written signature is checked where the
      call is written and pays nothing at run time, so
      `crates/nvs-ir/src/lower/expr.rs:5940`'s `CheckedTy::Callable` arm has nothing narrower than
      `CLOSURE_MARKER` (`crates/nvs-ir/src/lower/mod.rs:3617`) to test against. Each `fn` literal
      already gets a synthesized class of its own
      (`crates/nvs-ir/src/lower/closure.rs:296` is where it conforms to the marker), so class
      identity does decide the signature — the question is whether the row is the set of those
      class labels whose signature matches, or a signature the descriptor carries.
- [ ] **The `callable(...)` row over that decision** — `crates/nvs-ir/src/lower/expr.rs:5940` is the
      arm that answers bare `callable` today and where `CallableSig` joins it; the refusal at
      `crates/nvs-ir/src/lower/expr.rs:4986` loses its last shape when it lands.
- [ ] **The stage's case, extended** — the stage's check already names
      `tests/conformance/lang/an-is-test-against-iterable-or-callable-answers-by-what-the-value-holds.nvst:81`,
      where the disjointness sweep ends, and the signature rows belong after it rather than in a
      second file: what they pin is that bare `callable` and a written signature do not collapse
      (`crates/nvs-types/src/ty.rs:182`).

## Backlog

- Stage 8's declared-type rows, whichever of its two the probe proves unreachable — `docs/agent/loop-goal.md` § *Stage 8*.
- `docs/plan/m4.md`'s stale 1000-case figure and `done*` — goal `plan-truth` owns it.
