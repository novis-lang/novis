# Handoff

## State

**M4, Stage 00, and all six of its shapes are closed.** Item 48 landed this session together with the
three Stage 00 conformance cases whose names the gate held but the tree did not, so every Stage 00
check but item 49's is green.

`crates/nvs-types/src/expr/calls.rs`'s `check_new_target` now holds a `Core` `new` target to
`nvs_stdlib::registry` rather than to the `Core\` spelling. `QName::is_core` is a *spelling* test —
anything under `Core\` answers it — so accepting a target on that alone let `new Core\Bogus()` past
every check the checker has and fail in `nvs-codegen` with "this unit declares no descriptor for it",
an internal message for an ordinary typo. It is `E0303` now. The two `Core` mistakes stay distinct on
purpose: a *registered* class with no constructor keeps `infer_new`'s `E0405` naming the member it has
not got, because `nvs_stdlib::registry::CONSTRUCTORS` is the whole roster of names `new` may be written
on and "this class is not constructible" is a different fact from "this class does not exist". The
arity half of the same rule was already right and needed nothing — a scratch probe, not a reading of
the item, is what settled that.

**`Core` is two rosters and the registry is only the first.** `nvs_hir::errors::TREE`'s one namespaced
row, `Core\Test\Failure`, is trusted through `is_core` rather than through
`QName::is_reserved_global_class` — that predicate's own doc says so — so the arm carries
`errors::is_exception_class` beside the registry lookup, and
`the_namespaced_exception_tree_row_is_still_a_new_target` pins it. The playbook bullet under *Writing
Novis itself* owns the trap.

**Three conformance cases now pin work that had only unit tests.** Items 45, 46 and 47 landed their
`nvs-types` tests in earlier sessions but not their corpus, and `loop-goal.toml`'s Stage 00
`nvs-suite` block named all three files. They are under `tests/conformance/lang/` and each runs the
narrowed or substituted thing rather than only compiling it. One behaviour they pinned that no doc
stated: a nested `instanceof` guard replaces the residue rather than intersecting with it — the
playbook bullet under *Writing a test case* owns it.

`docs/agent/guard-name-debt.md` § *Stage 00's remaining names* is down to one bullet, item 49's, and
its intro count is rewritten to match.

## Next group

**Item 49, which is the last Stage 00 shape and is three edits in three files.** One file set:
`crates/nvs-ir/tests/type_atoms.rs`, `crates/nvs-ir/tests/refusals.rs`, `tools/holes.py`. Take them in
this order — the third is a *measurement* the second one records, so it cannot come first.

- [ ] **Widen the atom table to a roster of source shapes** (`loop-goal.md` item 49, ADR 0007 § 3).
      `crates/nvs-ir/tests/type_atoms.rs:232` is `every_spellable_type_reaches_a_diagnostic_or_an_ir`,
      which walks every *type* atom through two declaration positions. The sibling this owes is
      `every_spellable_expression_reaches_a_diagnostic_or_an_ir` — a class-constant read, a `new`, a
      call through each receiver kind, each statement form, each asserted to reach a diagnostic or an
      IR. That is the gate name still unwritten, and `guard-name-debt.md` names it.
- [ ] **Teach `tools/holes.py`'s `REFUSAL` to recognize a panic by shape, not by phrasing.**
      `tools/holes.py:53` is the recognizer's own comment: it is a three-phrase match over `panic!`,
      `assert!`'s second argument and `CodegenError::Unsupported`, which is why item 45's panic was
      invisible to it. Match the construct rather than the wording.
- [ ] **Re-derive `refusals.rs`'s `CEILING` from the true count, and say so in the commit.**
      `crates/nvs-ir/tests/refusals.rs` carries **4**, which the plan already records as a floor on
      the truth rather than the truth — it came from the blind count the recognizer above produced. A
      ratchet set from a blind count is not a ratchet.

## Backlog

- There is no intersection type, so two `instanceof` guards cannot both hold — decided nowhere yet;
  if it is ever wanted, it is an ADR, not a checker fix (`nvs_types::locals`' narrowing section).
- `new Core\Order()` on a `Core` *enum* now answers "`Core\Order` is not declared", which is true of
  the class position it was written in but says nothing about the enum (`nvs-types/src/expr/calls.rs`
  `check_new_target`).
- Stage 8's `refusals.rs` ceiling ratchets down and never up; a new refusal beside an old one in a
  claimed file is otherwise invisible (`loop-goal.md` § *Standing decisions*).
- ADR 0028 § 2's abandoned-generator `finally`, still unlanded (`loop-goal.md` § *Standing decisions*).
