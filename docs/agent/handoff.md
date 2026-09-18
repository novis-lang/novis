# Handoff

## State

**Goal `one-type-test`, stage 5's three acceptance checks are green.** `Core\Ast`'s roster is pinned
to one type-test node (`crates/nvs-stdlib/src/ast.rs:654`), go-to-definition on the type after `is`
answers the interface (`crates/nvs-lsp/src/definition.rs:958`), and the formatter lays a class
reference after `is` where a written type goes (`crates/nvs-fmt/tests/novis_constructs.rs:64`). All
three machineries were already landed by stages 3 and 4 — `named_at` reads `ExprInfo::TypeTest`, the
walk carries the value operand as the type test's second child, and the printer has no `is`-specific
arm at all — so these slices are the pins plus the prose those files still explained themselves by.

**What still spells the word in `crates/`.** The library prose — `crates/nvs-stdlib/src/debug.rs:70`
and its help string at `:381`, `crates/nvs-stdlib/src/reflect.rs:87`,
`crates/nvs-stdlib/src/lib.rs:270`, `crates/nvs-stdlib/src/instance.rs:418`, `:472`, `:508`, `:812` —
and the diagnostic help strings in `crates/nvs-types/src/expr/members.rs:883` and
`crates/nvs-types/src/expr/calls.rs:1165`, each frozen by a conformance case that moves in the same
slice. `crates/nvs-types/src/locals.rs` and `layout.rs` carry the rest, which is stage 6's and
stage 7's.

**Two holes the goal opened and nothing has closed.** `$m is Core\Str` type-checks and then dies at
codegen — the refusal reads *`is Core\Str`, whose class this unit declares no descriptor for*
(`crates/nvs-codegen/src/emit.rs:2678`) — and `rule:types/type-test` says the answer is `false`, so
the checker owes a fold. And go-to-definition answers nothing for any expression written inside a
function body, which is not this goal's doing; the playbook bullet is what a test case has to work
around.

## Next group

**Stage 5: the library prose that still explains itself by the old word** — one file set:
`crates/nvs-stdlib/src/debug.rs`, `crates/nvs-stdlib/src/reflect.rs`,
`crates/nvs-stdlib/src/instance.rs`, `crates/nvs-stdlib/src/lib.rs`. No acceptance check names these;
stage 7's `git grep -i -w instanceof` over `crates/` is what they are owed to.

- [ ] **`Core\Debug`'s union prose and its help string say `is`** — the module doc at
      `crates/nvs-stdlib/src/debug.rs:70` and the refusal help at `crates/nvs-stdlib/src/debug.rs:381`,
      whose text a conformance case freezes and so moves in the same slice.
      `rule:php-migration/one-type-test` is what it follows.
- [ ] **`Core\Reflect`'s subclass note says `is`** — `crates/nvs-stdlib/src/reflect.rs:87`, which
      explains a description by what `instanceof` would have answered.
      `rule:core-classes/reflect` owns the member it is about.
- [ ] **The class-identity prose in `instance` and the re-export beside it** —
      `crates/nvs-stdlib/src/instance.rs:418`, `:472`, `:508`, `:812` and
      `crates/nvs-stdlib/src/lib.rs:270`, each naming the old operator for the question
      `class_has_instances` answers. `rule:types/type-test`'s value arm is the test they describe.

## Backlog

- The checker owes `$m is Core\Str` a fold to `false` — `crates/nvs-codegen/src/emit.rs:2678`,
  `rule:types/type-test`.
- The diagnostic help strings at `crates/nvs-types/src/expr/members.rs:883` and
  `crates/nvs-types/src/expr/calls.rs:1165`, each with its conformance case.
- Stage 6: every `.nvst`, `.lspt` and Rust guard test that spells the word, renames included —
  `docs/agent/loop-goal.md` § *Stage 6*.
- Stage 7: the doc homes outside the code and the absence gate — `docs/agent/loop-goal.md` § *Stage 7*.
- Go-to-definition answers nothing inside a function body — `crates/nvs-lsp/src/definition.rs`.
