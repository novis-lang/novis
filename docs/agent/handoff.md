# Handoff

## State

**Goal `one-type-test` is met.** Stage 7's four checks are green: the gate's `instanceof` grep returns
nothing outside its exclusions, the retired rule id is cited nowhere, `docs/rules/php-migration.json:199`
reads `"status": "shipped"`, and `python tools/chain.py --check` walks 1..69. `python tools/verify.py` is
green (conformance 2060), `--doc` resolves every link, and neither `owners.py --closes one-type-test`
nor `playbook.py --closes one-type-test` names a gap.

**The codegen hole is closed in two places, and which one answers is a property of where the class is
written.** A test whose whole type names only classes nothing is an instance of folds to `false` in the
checker — `crates/nvs-types/src/expr/members.rs:290` is `names_no_instance`, read through `never_holds`
at `crates/nvs-types/src/expr/type_test.rs:286`. A `Core` namespace class in a position no fold reaches
— a union member beside a live one, an array element, a shape field — lowers to `nvs-ir`'s
never-matching row, so `[] is array<Core\Str>` is `true`, an empty array having no element to fail.
`rule:types/type-test` states both.

**What still reaches `crates/nvs-codegen/src/emit.rs:2681` is a `catch` clause's label alone**:
`caught_class_label` reads the written type as source text, so `use Core\Db\RolledBack;` with `catch
(RolledBack $e)` resolves to a name neither descriptor table holds. That is the only producer left, the
guard's comment now says so, and the playbook bullet owns the symptom.

**The word is gone from `crates/` and `tests/` outside the refusal's own homes**, test-function names
included, and the floor rows carried in `docs/agent/loop-goal.toml` name the tests that exist. One of
them wanted a retired diagnostic: the `holes.py --guarded` row's `want` no longer asks for `E0497`,
which nothing in the tree has emitted since the subject refusal was deleted.

## Next group

**Goal `gap-zero`'s own handoff takes over at the switch** (`docs/agent/goals/68-gap-zero.handoff.md`).
These two are what this tree still owes in the file set this session held — take them only if the run
stays on this goal:

- [ ] **A `catch` clause's label resolves its `use` alias before it becomes a class name**, so an
      aliased error class is the ordinary descriptor walk rather than a codegen refusal —
      `crates/nvs-ir/src/lower/exception.rs:800` is `caught_class_label`, which reads the clause's type
      as text. `rule:errors/throwable-hierarchy` is what a `catch` may name.
- [ ] **Decide whether `rule:types/type-test` flips from `designed` to `shipped`** — a claim about the
      operator's whole surface rather than about this hole, so it wants the table walked row by row
      first: `docs/rules/types.json:325`.

## Backlog

- `rule:types/type-test` is still `designed` while the divergence rule it is cited from is shipped —
  `docs/rules/types.json:325`.
- The `use`-alias `catch` label is a codegen refusal — `docs/agent/playbook.md:4258` holds the symptom,
  `crates/nvs-ir/src/lower/exception.rs:800` the cause.
- `docs/adr/README.md` still spells the removed keyword in its frozen prose, which is history rather
  than a rule — `docs/agent/loop-goal.md`'s gate excludes it on purpose.
