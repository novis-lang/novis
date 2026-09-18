# Handoff

## State

**Goal `one-type-test`: stages 1–6 are green and only stage 7, the gate, is red.** Stage 6's nine
conformance cases are all on disk and passing, so what is left is the sweep of the word out of
`crates/`, `docs/` and the remaining `tests/conformance` help strings. Stage 5 finished this session:
`crates/nvs-stdlib` no longer spells it anywhere.

**Two of those sentences were false rather than merely stale, and both were rewritten from a probe.**
`Core\Cli\Text|Core\Html\Markup $r = Core\Debug::render(1); if ($r is Core\Html\Markup)` compiles,
runs and narrows, so `Core\Debug`'s bound is now argued from the sink — `rendered_for` asks the
channel in force, so a caller narrowing that union would branch on the deployment's configuration —
in its home `docs/rules/errors/debug-dump.md:15` and restated in the module doc. And nothing on the
`is` path consults `crate::core_lib::has_instances` (`testable_core_class`'s only callers are
`crates/nvs-types/src/expr/operators.rs:1867` and `crates/nvs-ir/src/lower/closure.rs:387`), so
`crates/nvs-stdlib/src/instance.rs:468` now says *downcast target* where it said *`instanceof`'s
right-hand side*.

**The hole that leaves is unchanged and is stage 7's to close or to carry.** `$m is Core\Str`
type-checks and dies at codegen — `crates/nvs-codegen/src/emit.rs:2678` — where `rule:types/type-test`
says a knowable answer folds to `false`. The comment above it at `:2673` also names
`expr::members::testable_class_name`, a symbol that does not exist; the function is
`testable_core_class`.

## Next group

**Stage 7: the `nvs-types` help strings and the conformance cases that freeze them** — one file set:
`crates/nvs-types/src/expr/` plus `tests/conformance/`. Each help string is frozen verbatim by at
least one `.nvst`, so the string and its case move in the same slice or the suite goes red.
`rule:php-migration/one-type-test` is what all three follow.

- [ ] **The two erased-receiver help strings say `is`** — `crates/nvs-types/src/expr/members.rs:883`
      and `crates/nvs-types/src/expr/members.rs:888`, frozen by
      `tests/conformance/lang/a-class-name-constant-needs-a-class-the-compiler-resolves.nvst:40` and
      `:47`. The module doc above them, `crates/nvs-types/src/expr/members.rs:53` and `:452`, is the
      same slice's prose.
- [ ] **`calls.rs`'s three help strings and its two notes** — `crates/nvs-types/src/expr/calls.rs:1166`,
      `:1235` and `:1323`, frozen by
      `tests/conformance/lang/a-first-class-callable-names-a-member-not-a-class.nvst:42`,
      `tests/conformance/reject/a-call-through-a-mixed-receiver-refuses-what-it-cannot-defer.nvst:49`
      and `:56`, and
      `tests/conformance/lang/a-method-call-through-an-erased-receiver-is-a-diagnostic.nvst:30`; the
      notes are `crates/nvs-types/src/expr/calls.rs:275` and `:1307`.
- [ ] **`operators.rs`'s three `as`-into-a-class help strings** —
      `crates/nvs-types/src/expr/operators.rs:1560`, `:1884` and `:2246`, plus the note at `:1523`.
      Grep `tests/` for each string before editing it: these three may be unfrozen, unlike the six above.

## Backlog

- The rest of stage 7 in `crates/`: `nvs-types` `locals.rs`, `layout.rs`, `callables.rs`,
  `core_lib.rs:226`, `expr_table.rs:737`, `expr/mod.rs:97`, `type_test.rs:141` and its two test files.
- The rest of stage 7 in `crates/`: `crates/nvs-hir/src/errors.rs:92` and
  `crates/nvs-hir/src/members.rs:1185`.
- `docs/` is the other half of the gate: `docs/rules/` fragments (classes, core-api, expressions,
  types) then `python tools/rules.py --render`, plus `docs/reference/`, `docs/spec/` and the six
  `docs/agent/playbook.md` bullets.
- `crates/nvs-codegen/src/emit.rs:2673` names `expr::members::testable_class_name`, which is
  `testable_core_class` — a one-word fix outside this goal's file sets.
