# Handoff

## State

**Goal `one-type-test`, stage 3's code is landed and the test corpora are not.** `is` is the one
type test in `crates/`: `ExprKind::InstanceOf` is deleted, `ExprKind::TypeTest` carries
`against: TestOperand` (`Type` | `Value`), `instanceof` met where a binary operator may stand is
**`E0253`** and consumes its right operand, `infer_type_test` has the value arm recording
`ExprInfo::ClassRefTest { base }`, and `E0497`/`E0812` are retired with `E0496` renamed
`E_DYNAMIC_CLASS_NAME` and rewritten to its three sites. `cargo build --all-targets` is clean and
every Rust test in `nvs-syntax`, `nvs-types`, `nvs-ir`, `nvs-codegen`, `nvs-hir` and `nvs-fmt`
passes.

**What is red, and it is the whole of what is red.** 43 `.nvst` cases under `tests/conformance/`,
2 under `tests/differential/`, and `nvs-lsp --test coverage`, whose matrix now has a `TypeTest`
row with seven empty cells. Nothing is blocked and no decision is open: this is stage 6's
respelling, pulled to the front because stage 3 cannot be green without it. (`nvs-stdlib`'s
`socket_ping_keeps_a_quiet_live_peer_open` and `nvs-cli`'s `sd_notify_messages_…` failed under
load and passed alone in the same run — `verify.py` says so itself, and neither touches this work.)

**Two things went further than stage 3's prose, and both were forced.** `lower_type_test` gained
the descriptor arm now rather than at stage 4, because deleting `lower_instanceof` would otherwise
leave `$x is $cls` panicking at lowering; and `testable_core_class` is **kept** (the prose says
delete) because `nvs-ir`'s closure lowering and `nvs-types`' downcast rule both call it —
`testable_class_name` alone went. `InstKind::InstanceOf` still has its old name, which is stage 4's.

## Next group

**Stage 6 (front half): the corpus respelled** — one file set: `tests/conformance/`,
`tests/differential/`, `tests/lsp/`. Nothing in `crates/` is touched, and the gate is
`python tools/verify.py` green.

- [ ] **The conformance cases** — 43 files under `tests/conformance/` spelling `instanceof`,
      `git grep -l -w instanceof -- tests/conformance` being the list. `$x instanceof C` becomes
      `$x is C` and `$x instanceof $cls` becomes `$x is $cls`; the six `reject/` and
      `lang/instanceof-refuses-*` cases assert retired codes and are rewritten to what
      `crates/nvs-diagnostics/src/lib.rs:1331` (`E_DYNAMIC_CLASS_NAME`) now says, or deleted where
      the refusal itself is gone — `lang/instanceof-refuses-a-subject-that-can-hold-no-object.nvst`
      is the one whose whole subject retired. `rule:types/type-test` is the shape.
- [ ] **The differential pair** — `tests/differential/class/instanceof-matches-php.nvst:5` and
      `tests/differential/lang/instanceof-through-an-erased-subject-matches-phps.nvst:5`. Both
      carry an `--ORACLE--`, so the PHP side keeps `instanceof` and only the Novis side is
      respelled; check the pair still agrees rather than freezing a new expectation.
- [ ] **The `TypeTest` row of the LSP matrix** — `crates/nvs-lsp/tests/coverage.rs:41` names the
      seven empty cells: hover, completion, selectionRange, codeAction, references,
      documentHighlight at `TypeTest`, and one whole-document request reaching it.
      `tests/lsp/definition/an-instanceof-answers-the-interface-it-tests-against.lspt` is the
      model and the one that already passes.
- [ ] **The file renames** — every `instanceof-*.nvst` basename and
      `tests/lsp/definition/an-instanceof-answers-the-interface-it-tests-against.lspt:1`, patched
      everywhere they are named: `docs/rules/types/type-test.md:7`'s `guarded by`,
      `docs/rules/types/narrowing.md:8`, module docs, and the goal's own `.toml`. The goal's
      standing decisions make a rename whole only when `git grep` of the old basename is empty.

## Backlog

- Stage 4: `InstKind::InstanceOf` → `ClassTest`, `RuntimeSig`, `Sigs::instanceof`,
  `nvs_object_instanceof`/`nvs_value_instanceof`, the printer mnemonic (goal prose § *Stage 4*).
- Stage 5: `Core\Ast`'s roster still lists `"InstanceOf"` (`crates/nvs-stdlib/src/ast.rs:293`),
  `Core\Debug`/`Core\Reflect` docs, `crates/nvs-lsp/src/index.rs:886`'s `"InstanceOf"` key.
- Stage 7: the prose sweep, `status: shipped`, the `git grep -i -w instanceof` absence gate.
- `examples/objects.nvs` and `examples/targets.nvs` still spell `instanceof` and are compiled by
  the example gate.
- The website rule mirror is stale for the rules stage 2 touched; nothing gates it.
