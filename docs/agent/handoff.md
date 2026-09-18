# Handoff

## State

**Goal `one-type-test`'s floor is green again.** `examples/objects.nvs`, `examples/targets.nvs` and
`examples/type-test.nvs` spell the type test `is`; the stage-7 absence gate now greps `examples` too,
in both `docs/agent/loop-goal.toml:11626` and its byte-twin `docs/agent/goals/67-one-type-test.toml`.
That omission is why the word survived a goal whose gate was green. `python tools/verify.py` is green
across all 11 legs (conformance 2061, differential 279).

**A `catch` clause's class label is the `QName` the checker resolved, not the clause's source text.**
`crates/nvs-ir/src/lower/exception.rs:800` is `caught_class_label`, which reads
`ExprTypeTable::declared_ty(ty.span)` — the entry `nvs_types::lower::lower_type` persists for every
written annotation — and falls back to the text only when no checking pass ran. A name an import
brought into scope and a name written inside a `namespace` block both reach the descriptor table now;
`tests/conformance/lang/a-catch-clause-resolves-its-class-the-way-the-file-names-it.nvst` pins all
four spellings, block form and expression form.

**Nothing in the tree produces the codegen refusal at `crates/nvs-codegen/src/emit.rs:2681` any
more.** Its comment says so: every label arriving there is a rendered `QName`, so the `Unsupported`
return is a backstop against a unit built without the descriptor rather than a spelling this crate
cannot resolve.

## Next group

**Stage 7: the goal's last claim, one file set — `docs/rules/types.json` and the probe programs that
answer it.**

- [ ] **Walk `rule:types/type-test`'s *What may appear on the right* table row by row**, one probe
      program per row, and flip the rule's `status` from `designed` to `shipped` only if every row
      answers — `docs/rules/types.json:325` is the field. The claim is the operator's whole surface,
      not this goal's last hole, which is why it wants the table walked rather than asserted.
- [ ] **Decide whether the codegen guard stays a refusal or becomes an assertion** now that no
      producer reaches it — `crates/nvs-codegen/src/emit.rs:2681`. A `CodegenError::Unsupported` that
      no program can trigger is either a backstop worth its comment or a panic that says so louder;
      `rule:types/type-test` is what settles which.

## Backlog

- Goal `gap-zero`'s own handoff takes over at the switch — `docs/agent/goals/68-gap-zero.handoff.md`.
- The gate's path list still omits `docs/agent/goals/`, on purpose: a retired goal's prose names the
  word as history — `docs/agent/loop-goal.toml:11626`.
