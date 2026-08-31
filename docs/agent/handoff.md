# Handoff

## State

**Stage 10 item 38 has two of its three dynamic sites.** `new $cls(...)` lowers to
`InstKind::NewDynamic` with the operand's descriptor, and `$cls::f(...)` to one
`InstKind::CallVirtual` dispatching on it — both run end to end under `nvs run`, over a folded
`Dog::class as class<Animal>` and over a walked `$name as class<Animal>` alike, and an implementor
that overrides the member wins while one that does not falls back to the bound's body.

**The seam the last two sessions described is closed by two `ExprInfo` variants**, each recorded
where the guarded `New`/`Call` entry refused to be: `ExprInfo::NewDynamic { bound, ctor, ty }` and
`ExprInfo::ClassRefCall(ResolvedCall)` (`crates/nvs-types/src/expr_table.rs:362`). Their own doc
comments are the home for why each is a variant rather than a field on its sibling — in both cases
because the existing variant names a class a consumer may bind straight to, and the class here is
whichever implementor the descriptor holds.

**One refusal widened.** A non-`static` member reached through a `class<T>` class side now takes
`report_instance_method_called_statically` unconditionally, where a written class side still only
takes it with no `$this` in scope: `self::f()` is legal for an instance `f` because it forwards the
enclosing frame's receiver, and a class reference has none to forward. That is the sentence guarding
`lower_static_call_on_a_class_reference`'s `receiver: None`.

**Known gap, recorded on `ExprInfo::ClassRefCall`:** `$cls::f(...)` written as ADR 0027's
first-class callable still records `ExprInfo::CallableRef`, so the `Closure` names `T`'s method
rather than the implementor's. It predates this session and needs a refusal or a descriptor-carrying
closure, not a lowering.

**`as ?class<T>` still has no row** and still needs the representation decision ADR 0125 § 2
promises — unchanged, and `Lowering::lower_class_reference`'s *Known gaps* names the shape.

**`orient.py`'s `[context]` gaps.** Standing: no field selects `docs/reference/lang/*.md` (item 39
needs it); `docs/adr/README.md` and `ground-rules.md` are not in `modules`; the pack prints the goal
item but not the `[[check]]` grading it. `crates/nvs-host/src/budget.rs` is the documented forward
anchor. Nothing new was missing this session.

## Next group

**Item 38's last site, then item 39's corpus — file set `crates/nvs-types/src/expr/members.rs`,
`crates/nvs-types/src/expr_table.rs`, `crates/nvs-ir/src/ir.rs`, `crates/nvs-ir/src/lower/expr.rs`
and `crates/nvs-codegen/src/emit.rs`.** The instruction is the work: `InstKind::InstanceOf` takes a
`class: String` label today and § 4 needs a descriptor-valued form, which is five emit sites plus
codegen.

- [ ] **`$x instanceof $cls` tests two descriptors.** `infer_instanceof`
      (`crates/nvs-types/src/expr/members.rs:281`) already accepts the operand; the entry it records
      is `crates/nvs-types/src/expr_table.rs:666`. `InstKind::InstanceOf`
      (`crates/nvs-ir/src/ir.rs:792`) grows the descriptor form beside its label —
      `crates/nvs-ir/src/lower/expr.rs:4272` is the `$x instanceof C` emit and the four others
      (`lower/closure.rs:382`, `lower/convert.rs:1038`, `lower/exception.rs:182` and `:496`) keep
      the label. Codegen is `crates/nvs-codegen/src/emit.rs:3437`. ADR 0125 § 4.
- [ ] **The three positive `.nvst` cases the goal check names**, once the site above lands:
      `tests/conformance/class/a-class-reference-instantiates-the-subclass-it-names.nvst`,
      `…/a-class-reference-carries-a-static-call-and-an-instanceof.nvst` and
      `…/a-name-outside-the-hierarchy-throws-at-the-conversion.nvst`. Item 39; every shape they
      need is already known to run, and `crates/nvs-ir/tests/class_reference.rs:171` is the
      hierarchy each was exercised over.
- [ ] **The reject case**,
      `tests/conformance/reject/a-subclass-with-another-constructor-refuses-a-dynamic-new.nvst`,
      over the `E0794` at `crates/nvs-types/src/expr/calls.rs:653` — `--EXPECTF-ERROR--`, whose
      indentation widens with the line number.

## Backlog

- `as ?class<T>` needs a representation decision — `nvs-ir`'s `lower_class_reference` *Known gaps*.
- `$cls::f(...)` as a first-class callable names the bound's method — `ExprInfo::ClassRefCall`.
- Stage 8: conformance 1087, differential 206 of 210, migration 37% — `docs/plan/m6.md`.
- `orient.py` `[context]` has no `docs/reference/lang/*.md` selector — `docs/agent/loop-goal.toml`.
