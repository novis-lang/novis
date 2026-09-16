# Handoff

## State

**Goal 63 — a `Core` class is a name a type test can walk — has just started; nothing of it has landed yet.** Goal `worker-placement`'s whole list is this goal's Stage 1 floor.

Both halves of the answer are on disk and neither is this goal's to build. The descriptor is published —
`nvs_stdlib::instance::class_descriptors` (`crates/nvs-stdlib/src/instance.rs:488`) hands the backend a
`(name, *const ClassDesc)` pair per `Core` class — and the run-time lookup is built:
`nvs_runtime::Ctx::class_desc` (`crates/nvs-runtime/src/ctx/error.rs:261`) asks the `Core` resolver after
the program's own table, which is why a decoded `Core\Time\Date` already arrives back under its own
descriptor. What a session must not re-decide: no second descriptor table, and
`string as Core\Html\Markup` stays `lower_markup_lift`'s lift rather than becoming a downcast
(the goal's *Standing decisions*).

## Next group

**Stage 2: the testable name** — one file set: `crates/nvs-types/src/expr/members.rs`,
`crates/nvs-types/src/expr_table.rs`, `crates/nvs-stdlib/src/instance.rs`.

- [ ] **A `Core` class stops being an untestable right-hand side** —
      `crates/nvs-types/src/expr/members.rs:298`'s `infer_instanceof` reports
      `E_INSTANCEOF_NOT_A_CLASS` for a `Core` class because it "has no descriptor laid out for the test
      to walk" (that module's docs, `:44-49`). Three refusals share the code and only this one moves:
      the dynamic form and the enum stay. `rule:types/class-reference-sites` is what bounds a written
      class name at each site that takes one.
- [ ] **The test is recorded with its resolved name** — `crates/nvs-types/src/expr_table.rs:794`'s
      `ExprInfo::InstanceOf` is recorded for a class the program declares and never for the dynamic
      form; a `Core` name resolves through the same namespace and import rules, so it is recorded the
      same way and `nvs-ir` needs no new shape to read.
- [ ] **The lowering reaches the published address** — `crates/nvs-ir/src/lower/expr.rs:6213`'s walk
      turns a recorded test into a `TestShape`; a `Core` class's descriptor comes from
      `crates/nvs-codegen/src/lib.rs:576`'s `class_desc(label)` over the pairs
      `crates/nvs-stdlib/src/instance.rs:488` already publishes. Nothing new is registered.

## Backlog

- **Stage 3, the conversion target** — `crates/nvs-types/src/expr/operators.rs:1765`'s
  `reject_unconvertible` and `crates/nvs-ir/src/lower/convert.rs:1171`'s `lower_checked_downcast`:
  `mixed as Core\Time\Date` becomes the checked downcast over the descriptor stage 2 made reachable,
  while `object`, a shape and a `callable` stay refused. Cheap right after stage 2.
- **Stage 4, the three spellings** — `rule:types/type-test` owns `is`, and whatever it already answers
  for a `Core` class is what `instanceof` and `as` must agree with. Read it before writing the cases.
- **Stage 0's four sentences** — each is rewritten in the slice that makes it wrong, not after it;
  the goal's *Stage 0* lists them with their files.
- **`crates/nvs-runtime/src/graph.rs`'s `# Known gaps` block** is what this goal closes, rewritten as
  what the round trip then does.
- When this goal's last check goes green the driver takes goal `gap-zero`.
