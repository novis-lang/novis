# Handoff

## State

**Goal `one-type-test`, stage 4 is landed.** The word `instanceof` now names nothing at all in
`crates/nvs-ir`, `crates/nvs-codegen` or `crates/nvs-runtime`. The names it left behind:
`InstKind::ClassTest` (`crates/nvs-ir/src/ir.rs:1213`), the printer mnemonic `class.test`,
`RuntimeSig::ClassTest`, `Sigs::class_test`, `Emitter::emit_class_test`
(`crates/nvs-codegen/src/emit.rs:2661`), and the two runtime entry points `nvs_object_is_class` /
`nvs_value_is_class` (`crates/nvs-runtime/src/object.rs:3939`). `TestedClass` keeps its name, as the
goal says. Both of stage 4's acceptance tests exist and pass.

**What still spells the word in `crates/`, and it is all stage 5 and the diagnostics.** The library
and the LSP prose (`crates/nvs-stdlib/src/{ast,debug,reflect,instance,lib}.rs`,
`crates/nvs-lsp/src/definition.rs:13`, `crates/nvs-fmt/tests/novis_constructs.rs:26`), and the
diagnostic *help* strings in `crates/nvs-types/src/expr/members.rs:883` and
`crates/nvs-types/src/expr/calls.rs:1165`, each frozen by a conformance case that moves in the same
slice. `ExprInfo::InstanceOf` is already gone from the LSP — stage 5's work there is the new
navigation test and the prose, not an arm to delete.

**Two holes the goal opened and nothing has closed.** `$m is Core\Str` type-checks and then dies at
codegen — the refusal now reads *`is Core\Str`, whose class this unit declares no descriptor for*
(`crates/nvs-codegen/src/emit.rs:2678`) — and `rule:types/type-test` says the answer is `false`, so
the checker owes a fold. And `crates/nvs-codegen/src/emit.rs`'s `emit_class_desc_in` carried the
*other* emitter's opening paragraphs as its own doc comment; that is repaired, but it is worth
knowing the file has had a doc comment attached to the wrong item before.

## Next group

**Stage 5: the library, the LSP and the formatter** — one file set: `crates/nvs-stdlib/src/ast.rs`,
`crates/nvs-lsp/src/definition.rs`, `crates/nvs-lsp/tests/navigation.rs`,
`crates/nvs-fmt/tests/novis_constructs.rs`. These are the three checks the driver reports next, and
stage 4 is out of their way.

- [ ] **`Core\Ast`'s roster answers with one type-test node** — the acceptance test
      `the_ast_roster_names_a_type_test_and_no_second_node_for_a_class_test` over the roster at
      `crates/nvs-stdlib/src/ast.rs:327`, whose `"TypeTest"` row is already the only one.
      `rule:core-classes/ast-is-inert` is what it pins, and the module doc at
      `crates/nvs-stdlib/src/ast.rs:43` still explains itself by `instanceof`.
- [ ] **Go-to-definition on the type after `is`** — the acceptance test
      `definition_on_the_type_after_is_answers_the_interface_it_tests_against` beside the existing
      navigation tests at `crates/nvs-lsp/tests/navigation.rs:112`, resolving through the type node
      `crates/nvs-lsp/src/definition.rs` already resolves for a declaration; its module doc at
      `crates/nvs-lsp/src/definition.rs:13` names `instanceof` and is the same slice's rewrite.
- [ ] **The formatter puts `is $cls` where `is T` goes** — the acceptance test
      `a_class_reference_after_is_is_formatted_as_a_type_is` beside the qualifier fixture at
      `crates/nvs-fmt/tests/novis_constructs.rs:21`, whose comment at
      `crates/nvs-fmt/tests/novis_constructs.rs:26` is rewritten with it.

## Backlog

- `$m is Core\Str` dies at codegen; the checker owes the `false` fold — `crates/nvs-codegen/src/emit.rs:2678`.
- `crates/nvs-stdlib/src/{debug,reflect,instance,lib}.rs` prose still says `instanceof` — stage 5's tail.
- `crates/nvs-types/src/{locals,layout,expr_table}.rs` prose is what stage 7's gate will find last.
- `docs/adr/README.md:261` names the retired symbol `nvs_object_instanceof`; left as frozen rationale.
