# Handoff

## State

**`Core\Ast::parse` answers the compiler's own tree**, and the walk behind it is
`nvs_syntax::walk` (`crates/nvs-syntax/src/walk.rs`) rather than anything in `nvs-stdlib`: the AST's
enums are `#[non_exhaustive]`, so only a match inside `nvs-syntax` is checked. What crosses the
crate boundary is `walk::Node`, a rose tree of `&'static str` kinds holding no `Expr`, no `Span` and
no borrow of the source — which is how ADR 0019 § 3's inertness is structural here rather than
promised. `crates/nvs-stdlib/src/ast.rs`'s module doc is the home of that argument and of the three
known gaps (the typed per-production roster, a node's own text, `parseFile`).

**A node is a statement, an expression, or a member of a declaration** — `walk`'s own module doc
owns why the line is drawn there, and `nodes()` is `children()` closed transitively with the
receiver excluded, which is what makes the fixture's count 4.

**The driver's acceptance check for stage 8 is green**: `examples/reflect.nvs` prints all five
lines, `Core\Arr::count($tree->nodes())` included.

**Stage 8's remaining named tests are the two reflective *actions*** — reflection can describe and
read today, and `crates/nvs-stdlib/src/reflect.rs`'s known gap 3 is the home of what calling and
writing still owe (ADR 0014's hook among them). Stage 7 still owes the schema-identity test in the
backlog.

## Next group

**The reflective actions — stage 8's last item, over `crates/nvs-stdlib/src/reflect.rs`,
`crates/nvs-runtime/src/ctx.rs` and `tests/conformance/core/`. ADR 0019 § 2 is the specification and
`docs/agent/loop-goal.toml:2467` is the check that names both tests.**

- [ ] **`Core\Reflect\ClassInfo::call` — a reflective call faces the visibility check the ordinary
      call faces** — ADR 0019 § 2. The reader half is already written against a descriptor:
      `crates/nvs-stdlib/src/reflect.rs:402`'s `describe` and `crates/nvs-stdlib/src/reflect.rs:600`'s
      `get` are the shape, and the dispatch a native member reaches a program's method through is
      `nvs_runtime::call_static` via `Ctx::class_desc` — `crates/nvs-stdlib/src/command.rs`'s
      `dispatching` test is the worked example. The named test is
      `a_reflective_call_to_a_private_method_from_outside_fails_like_the_ordinary_call`.
- [ ] **A reflective property write runs the hook an ordinary write runs** — ADR 0014, whose rule is
      that a property access runs its own hook. `crates/nvs-stdlib/src/reflect.rs:600`'s `get` is the
      read this mirrors, and `crates/nvs-stdlib/src/reflect.rs:81` (known gap 3) is the home of what
      the write owes beyond it. The named test is
      `a_reflective_property_write_runs_the_hook_an_ordinary_write_runs`.
- [ ] **Three `.nvst` cases per new member, under `tests/conformance/core/`** — the floor gate at
      `crates/nvs-stdlib/tests/conformance_coverage.rs:331` wants three *questions*, not three
      calls, and the error-path gate at `crates/nvs-stdlib/tests/conformance_coverage.rs:637` wants
      each new `Fault::` message either echoed by a case or declared unreachable at its site.
      `tests/conformance/core/core-ast-parse-stops-where-the-compiler-stops.nvst` is the shape.

## Backlog

- Stage 7's `application_code_and_the_engine_floor_produce_schema_identical_records` —
  `docs/adr/0020-error-escalation-ladder.md` § 6.
- ADR 0019 § 3's typed node roster (`Core\Ast\ClassDecl` and siblings) — known gap 1 in
  `crates/nvs-stdlib/src/ast.rs`.
- A node's own source text and position — known gap 2 there, and the point at which `parse`'s
  `$source` becomes `Qual::Contagious`.
- `Core\Ast::parseFile`, which is `fs.read` and so a capability-bearing member — known gap 3.
- M8's fuzz target for `Core\Ast::parse` over M1's parser corpus — `docs/plan/m8.md` § *Verify*.
- Item 35's `property<T>` ADR slot, the goal's one open ADR number.
