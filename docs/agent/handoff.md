# Handoff

## State

**Stage 8 has opened.** `Core\Reflect::forObject` answers a `Core\Reflect\ClassInfo`, and that
description's `name()`, `properties()` and `get($object, $name)` are rows, cards, bodies and
`.nvst` cases (`crates/nvs-stdlib/src/reflect.rs`, whose module doc is the home of why the
description holds *answers* rather than the described object, and why `get` refuses a value of
another class).

**A field slot's visibility is now a carried bit, not a recomputed one.** `nvs_types::layout` decides
it where the keyword still exists, `nvs_ir::ir::Class::public_fields` carries it, `nvs-codegen`
installs it and `nvs_runtime::ClassDesc::field_is_public` answers it — ADR 0092 § 5's `secret` bit's
plumbing exactly, and each of those four doc comments owns its own half. An unclassified slot reads
as **unreadable**, which is the opposite fallback from `field_is_secret`; `field_is_public`'s doc
says why both directions are the safe one.

**The driver's acceptance check is still red, and still is not a regression.**
`examples/reflect.nvs` now compiles as far as line 53 and stops at `Core\Reflect::typeOf`, which is
the next group's first item. Two wrong spellings in that fixture were fixed on the way past
(`->name` as a property, `Core\Arr::length`) — see the new playbook bullet.

**Stage 7 still owes one item**, the schema-identity test in the backlog below.

## Next group

**Stage 8's remainder — `Core\Reflect`'s last member, its enum, and `Core\Ast`, over
`crates/nvs-stdlib/src/reflect.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/lib.rs`, a new `crates/nvs-stdlib/src/ast.rs` and `tests/conformance/core/`.
Stage 8's rules and the fixture's five frozen lines are `docs/agent/loop-goal.toml:2463`.**

- [ ] **`Core\Reflect::typeOf(mixed): Core\Reflect\TypeKind`, and the enum it answers with** — the
      spec's single replacement for PHP's fourteen `is_*` predicates plus `gettype`
      (`docs/spec/01-core-library.md:964`). The member is a fifth row on
      `crates/nvs-stdlib/src/reflect.rs:83`; the enum is one line on
      `crates/nvs-stdlib/src/registry.rs:1545`'s `ENUMS` beside its `EnumDoc`. The goal check names
      `type_of_is_the_single_replacement_for_the_is_predicates`, and the fixture's fourth frozen line
      is `examples/reflect.nvs:53` — its `match` arms fix the case spellings (`Int`, `Float`, `Text`).
- [ ] **`Core\Ast::parse` as inert typed data** — ADR 0019 § 3, and ADR 0052's door stays shut, so
      the answer is data with no path back into execution. A new module registered on
      `crates/nvs-stdlib/src/registry.rs:1054`, over `nvs_syntax`'s own entry point
      `crates/nvs-syntax/src/parser/mod.rs:504`. The fixture asks `$tree->nodes()` and counts four at
      `examples/reflect.nvs:63`; the goal check is
      `a_parsed_ast_is_inert_data_with_no_path_back_into_execution`.
- [ ] **`Core\Reflect::forClass`** — the spec row's other door
      (`docs/spec/01-core-library.md:954`), reaching the compiled unit's class table rather than an
      instance's descriptor. Same five edits on `crates/nvs-stdlib/src/reflect.rs:83`; the route from
      a native member to a program's class is `Ctx::class_desc`, whose own doc comment says it is
      reached through `Ctx::set_runtime_error_class`'s handle and nothing else.

## Backlog

- Stage 7's last item: application code and the engine floor produce schema-identical log records —
  `docs/agent/loop-goal.toml:2440`.
- A reflective *call* to a `private` method, and a reflective property *write* running ADR 0014's
  hook — `crates/nvs-stdlib/src/reflect.rs`'s known gap 3; two of stage 8's named checks.
- `Core\Decimal`'s `divExact`/`divRound` split — stage 8's check
  `div_exact_throws_where_div_round_rounds`, ADR 0054.
- Item 35's `property<T>` and its ADR, the goal's one ADR slot — `docs/agent/loop-goal.toml:2500`.
- § 1's remaining `*Info` classes (`MethodInfo`, `ParameterInfo`, …) —
  `crates/nvs-stdlib/src/reflect.rs`'s known gap 1.
