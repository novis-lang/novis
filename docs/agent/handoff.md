# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4, 5 and 6 are done; stage 7 (`Core\Ast`) is open** and is
the driver's failing acceptance check.

A method's parameter names now travel `field_types`' road: `nvs_types::layout::ClassLayout::methods`
carries them as a fourth tuple element, `nvs_ir::ir::Class::methods` copies it down, both codegen
binders join it onto `nvs_runtime::MethodRow::param_names`, and `Core\Reflect\MethodInfo::parameters`
answers `array<Core\Reflect\ParameterInfo>` off it. An **empty** list reads as "no declaration was
read for this row" — a synthesized member (exception constructor, delegation forward, closure and
generator class) and every native row — never "takes nothing", and `parameterCount` is what still
answers for one. `crates/nvs-types/src/layout.rs:101` is the home of that reading.

`Core\Reflect\ParameterInfo` is registered and carries the name alone: a parameter's declared *type*
has no road to the descriptor yet. `NOT_YET_BUILT` (`crates/nvs-stdlib/src/reflect.rs:2237`) is down
to `ConstantInfo`, `AttributeInfo`, `EnumInfo`, and is still two-sided — registering one fails the
gate until its line is deleted. Nothing is blocked.

## Next group

**Stage 7: `Core\Ast` — the typed roster, the two cases and the fuzz target** — one file set:
`crates/nvs-stdlib/src/ast.rs`, `crates/nvs-syntax/src/walk.rs`, `tests/conformance/core/` and
`fuzz/`. `rule:core-classes/ast-is-inert` is the rule; ADR 0019 § 3 the record; the goal's
§ *Standing decisions* pre-authorizes the shape.

- [ ] **A typed class per production, replacing the one untyped node** — `crates/nvs-stdlib/src/ast.rs:127`
      is `NODE`, whose `kind()` names the production rather than being one, and gap 1 at
      `crates/nvs-stdlib/src/ast.rs:46` owns why. The roster is generated from
      `nvs_syntax::walk`'s production table — `crates/nvs-syntax/src/walk.rs:99` is `Node`,
      `crates/nvs-syntax/src/walk.rs:175` the entry point — and the acceptance's
      `every_production_the_walk_names_has_a_typed_ast_class` (new, `cargo test -p nvs-stdlib`) is
      what holds the two together.
- [ ] **The two `.nvst` cases the acceptance names** —
      `tests/conformance/core/ast-parse-answers-a-typed-node-per-production.nvst` and
      `tests/conformance/core/ast-parse-file-reads-through-the-io-door-under-fs-read.nvst`. Gap 3's
      `Decided:` sentence at `crates/nvs-stdlib/src/ast.rs:62` **strikes** the spec § 3 `parseFile`
      row, so the second case composes `Core\IO::read` with `parse` under `fs.read` rather than
      calling a capability-bearing member; striking the spec row is part of that slice.
- [ ] **The `ast` fuzz target and the seed replay beside it** — `fuzz/Cargo.toml:45` is the last
      `[[bin]]` block and `fuzz/fuzz_targets/parse.rs` the model. The acceptance runs `cargo
      metadata` over the manifest, never nightly, and
      `core_ast_parse_gives_the_compilers_verdict_on_every_parse_seed` (new, `-p nvs-stdlib`) replays
      the seeds on stable through `Core\Ast::parse`.

## Backlog

- A parameter's declared type and its default have no road to the descriptor; `ParameterInfo` would
  read them where it reads the name (ADR 0019 § 1, `crates/nvs-types/src/layout.rs:101`).
- `Core\Reflect`'s remaining roster classes — `ConstantInfo`, `AttributeInfo`, `EnumInfo` — are
  listed in `NOT_YET_BUILT` and owned by ADR 0019 § 1 and § 4.
- Stage 8 and later of this goal are untouched; `docs/agent/loop-goal.toml` is the list.
