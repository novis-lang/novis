# Handoff

## State

**Goal `one-type-test`, stage 6 is landed and the corpus is green.** No `.nvst` or `.lspt` case
spells `instanceof` any more except the one that refuses it: `tests/conformance/` is 2059 cases,
`tests/differential/` 279 (the two PHP twins are deleted — PHP cannot run `is`), and the
`nvs-lsp` coverage matrix has no empty cell. Two one-line repairs in `crates/` went with it
because nothing else could be green underneath them: `Core\Ast`'s roster no longer carries an
`InstanceOf` production (`crates/nvs-stdlib/src/ast.rs:293`), and `nvs_lsp::index::named`'s dead
`"InstanceOf"` arm is gone.

**What the word still names in `crates/`, and it is all that is left before stage 7's gate.** The
primitive keeps its old name — `InstKind::InstanceOf`, `emit_instanceof`,
`nvs_object_instanceof` / `nvs_value_instanceof` — which is stage 4's rename and the next group.
Five diagnostic *help* strings still say `instanceof`
(`crates/nvs-types/src/expr/members.rs:883` and `crates/nvs-types/src/expr/calls.rs:1165`), each
frozen by a conformance case that has to move in the same slice; that is stage 5's.

**One hole the goal opened and nothing has closed.** `$m is Core\Str` type-checks and then dies at
codegen — *nvs-codegen does not lower `instanceof Core\Str`, whose class this unit declares no
descriptor for yet* (`crates/nvs-codegen/src/emit.rs:2679`). `instanceof` refused that name at
check time; `is` resolves it as a type and `rule:types/type-test` says the answer is `false`, so
the checker owes a fold. It is in the backlog rather than the group because no case reaches it.

## Next group

**Stage 4: the primitive renamed, and the two tests the driver is waiting on** — one file set:
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/emit.rs`,
`crates/nvs-codegen/src/lib.rs`, `crates/nvs-runtime/src/helpers.rs`. This is the earliest-stage
red the driver reports, and stage 6 is out of its way now.

- [ ] **The IR instruction takes the name the operator has** — `InstKind::InstanceOf` becomes the
      class-test spelling at `crates/nvs-ir/src/ir.rs:1191`, with its two construction sites at
      `crates/nvs-ir/src/lower/expr.rs:5182` and `crates/nvs-ir/src/lower/expr.rs:5318`.
      `rule:types/type-test` § *The value arm* is what the descriptor arm implements.
- [ ] **`a_class_reference_test_lowers_to_a_descriptor_valued_class_test`** — the `nvs-ir` test the
      acceptance check names, over `crates/nvs-ir/src/lower/expr.rs:5141`'s value arm: a
      `class<T>` operand lowers to a class test whose target is the descriptor the reference
      carries, not a baked-in address.
- [ ] **`a_class_test_target_is_a_relocation_not_an_immediate`** — rename
      `crates/nvs-codegen/src/lib.rs:2811`'s `an_instanceof_target_is_a_relocation_not_an_immediate`
      and move `emit_instanceof` to `emit_class_test` at `crates/nvs-codegen/src/emit.rs:2662`.
      The check names only the new test name, so the old one cannot stay beside it.
- [ ] **The runtime symbols** — `nvs_object_instanceof` and `nvs_value_instanceof` become
      `nvs_object_is_class` and `nvs_value_is_class`, registered at
      `crates/nvs-runtime/src/helpers.rs:2983`, with `crates/nvs-runtime/src/object.rs`'s
      definitions and `crates/nvs-codegen/src/lib.rs:1786`'s signature doc moving with them.

## Backlog

- `$m is Core\Str` panics at codegen instead of folding to `false` — `crates/nvs-codegen/src/emit.rs:2679`, and `rule:types/type-test` says the answer is knowable.
- Five diagnostic help strings still name `instanceof` — `crates/nvs-types/src/expr/members.rs:883`, `crates/nvs-types/src/expr/calls.rs:1165`; stage 5 and the stage-7 gate.
- `nvs-fmt` names neither node kind anywhere, so `a_class_reference_after_is_is_formatted_as_a_type_is` is a spacing rule plus its test — stage 5.
- `docs/decisions/0150.md:140` names three case basenames this session renamed; a record is frozen rationale and outside stage 7's gate, so it was left alone.
- The website mirror under `website/src/content/docs/docs/rules/` still carries the old case paths and a stale "five spellings" for `types/narrowing`; nothing in `tools/` regenerates it.
- `docs/agent/goals/33-type-test.md` names `an-instanceof-narrows-its-subject-on-the-true-edge`, a goal's own prose and therefore history — `docs/agent/conventions.md` has no rule asking for it to move.
