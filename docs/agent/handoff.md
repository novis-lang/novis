# Handoff

## State

**`Core\Reflect` is three doors over one walk.** `forObject` reaches a descriptor through a
value, `forClass` reaches one through the running program's class table
(`nvs_runtime::Ctx::class_desc`), and both build the description in `describe`
(`crates/nvs-stdlib/src/reflect.rs`) so that what a description *is* cannot depend on which door
was used. `forClass` answers `?ClassInfo`: the `null` is the whole of `class_exists`, and a
`Core` class is not among the answers.

**`typeOf` answers `Core\Reflect\TypeKind`**, ten cases partitioning `nvs_runtime::Tag`'s
value-carrying half, registered on `nvs_stdlib::registry::ENUMS`. The module doc is the home of
why there is no `Callable`, `Numeric`, `Iterable` or `Countable` case; `kind_of`'s doc is the
home of why exactly three tags have none.

**The driver's acceptance check is still red, and still is not a regression.**
`examples/reflect.nvs` now runs through line 59 and stops at `Core\Ast::parse`, which is the next
group's first item and the fixture's last two frozen lines.

**Stage 7 still owes one item**, the schema-identity test in the backlog below.

## Next group

**`Core\Ast` — stage 8's second half, over a new `crates/nvs-stdlib/src/ast.rs`,
`crates/nvs-stdlib/src/registry.rs:1054`, `crates/nvs-stdlib/src/lib.rs:249` and `:359`, and
`tests/conformance/core/`. ADR 0019 § 3 is the specification and `docs/agent/loop-goal.toml:2463`
is stage 8's own rules.**

- [ ] **`Core\Ast::parse(string): Core\Ast\Node` as inert typed data** — ADR 0019 § 3, whose
      mechanism is not illustrative: it calls `crates/nvs-syntax/src/parser/mod.rs:504`'s
      `parse_file`, because there is no second grammar in this project. Register the class at
      `crates/nvs-stdlib/src/registry.rs:1054` and the module and its `address` arm at
      `crates/nvs-stdlib/src/lib.rs:249` and `crates/nvs-stdlib/src/lib.rs:359`. The fixture's
      frozen shape is `examples/reflect.nvs:62` — `Core\Arr::count($tree->nodes())` is **4** for
      `<?nvs echo 1 + 2;`, so `nodes()` answers a walk rather than the root's children alone.
      `crates/nvs-stdlib/src/reflect.rs:122`'s `CLASS_INFO` is the shape a `Core`-owned answer
      class takes, slots and readers included.
- [ ] **`a_parsed_ast_is_inert_data_with_no_path_back_into_execution`** — the goal check's own
      test name, in the new module's `mod tests`; ADR 0019 § 3 and ADR 0052's closed door on
      `eval` are what it pins. Anchor: `crates/nvs-stdlib/src/lib.rs:359`.
- [ ] **The two reflective-*action* checks** —
      `a_reflective_call_to_a_private_method_from_outside_fails_like_the_ordinary_call` and
      `a_reflective_property_write_runs_the_hook_an_ordinary_write_runs`, ADR 0019 § 2 plus ADR
      0014's hook. They are `CLASS_INFO`'s next two instance rows at
      `crates/nvs-stdlib/src/reflect.rs:122`, and known gap 3 in that module's doc is where the
      absence is currently recorded.

## Backlog

- Stage 7's `application_code_and_the_engine_floor_produce_schema_identical_records` —
  `docs/agent/loop-goal.toml` stage 7.
- `Core\Ast::parseFile` — ADR 0019 § 3 names it beside `parse`; it needs ADR 0118's `io` door.
- Method reflection (`get_class_methods`, `method_exists`) — `crates/nvs-stdlib/src/reflect.rs`
  known gap 1.
- Item 35's `property<T>`, the goal's one ADR slot — `docs/agent/loop-goal.md`.
- Stage 8's `nvs-types` half, the four attribute-retrieval fold checks — ADR 0046 §§ 4-6.
