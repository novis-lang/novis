# Handoff

## State

**Goal `one-type-test`: stages 1–6 are green, stage 7 — the gate — is red on two of its four checks.**
The word sweep is the visible one; the other is `docs/rules/php-migration.json:199`, where the
divergence rule still reads `"status": "designed"` and the gate wants `shipped`. The chapter and
`php-migration.md` are generated from the fragments and that json, so both are `python tools/rules.py
--render`'s output and never hand-edited (`tools/rules.py`'s module doc).

**`crates/nvs-types/src/expr/` is clean.** Every help string in `members.rs`, `calls.rs` and
`operators.rs` names `is`, and the five conformance cases that freeze them verbatim moved in the same
commits. What was stale rather than merely misspelled, and is now rewritten from the code: the
`members.rs` module doc described `infer_instanceof`, `E0497` and a right-hand-side rule that
`crates/nvs-types/src/expr/type_test.rs` has owned since the checker moved — neither the function nor
the code exists — and `calls.rs` cited `crate::locals::instanceof_residue`, which is
`crates/nvs-types/src/locals.rs:475`'s `type_test_residue`. What members.rs still owns is
`reject_dynamic_class_name`, the one `E0496` the three value spellings share.

**The hole from the last session is unchanged and is still stage 7's to close or to carry.** `$m is
Core\Str` type-checks and dies at codegen — `crates/nvs-codegen/src/emit.rs:2678` — where
`rule:types/type-test` says a knowable answer folds to `false`. The comment at `:2673` names
`expr::members::testable_class_name`, which does not exist; the function is `testable_core_class`.

## Next group

**Stage 7: the rest of `crates/nvs-types`, prose only** — one file set: `crates/nvs-types/src/` plus
`crates/nvs-types/tests/`. None of these lines is a diagnostic string, so no `.nvst` moves with them
and the suite cannot go red on the wording; `rule:types/type-test` and `rule:types/narrowing` are what
they follow. Two of them *compare* the two spellings, which § *Standing decisions* closes outright —
cite `rule:php-migration/one-type-test` rather than naming the word.

- [ ] **`locals.rs`'s narrowing prose** — `crates/nvs-types/src/locals.rs:29`, `:35`, `:38`, `:67`,
      `:72`, `:381`, `:386`, `:627`. The module doc's own heading lists the refused word beside `is`
      as two separate spellings, where `rule:types/narrowing` has four and `is` is the general one.
- [ ] **`layout.rs`'s four descriptor-edge comments** — `crates/nvs-types/src/layout.rs:125`, `:463`,
      `:595`, `:1169`. Each names the operator to say which edges a descriptor needs;
      `rule:types/type-test` is the same test under its living name.
- [ ] **The seven one-line citations** — `crates/nvs-types/src/callables.rs:21`,
      `crates/nvs-types/src/core_lib.rs:226`, `crates/nvs-types/src/expr/mod.rs:97`,
      `crates/nvs-types/src/expr_table.rs:737`, `crates/nvs-types/src/expr/type_test.rs:141`,
      `crates/nvs-types/tests/classes.rs:319`, `crates/nvs-types/tests/type_test.rs:182`.

## Backlog

- `crates/nvs-hir/src/errors.rs` and `crates/nvs-hir/src/members.rs`, one line each — the last two in
  `crates/` outside the gate's exclusions.
- `docs/rules/`: 11 lines across 10 fragments, including
  `docs/rules/php-migration/every-divergence-is-deliberate-and-listed.md`, which the gate does **not**
  exclude even though the chapter it renders into is excluded — it has to point at
  `rule:php-migration/one-type-test` instead of spelling the word. `python tools/rules.py --render`
  after each.
- `docs/spec/02-php-migration.md` (4), `docs/reference/tools/30-php-differences.md` (4),
  `docs/spec/01-core-library.md`, `docs/reference/findings.md` (2).
- `docs/agent/playbook.md` (11) and `tests/conformance/reject/new-takes-a-class-name-not-a-string.nvst`
  (2) — the playbook's are in bullets whose trailers may already have fired.
- `docs/rules/php-migration.json:199` → `"status": "shipped"`, and re-render. Last, with the sweep done.
- `crates/nvs-codegen/src/emit.rs:2678`'s `Core`-class fold, above.
