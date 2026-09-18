# Handoff

## State

**Goal `one-type-test` is met.** `is` is the one type test, `instanceof` is refused naming it, the
absence gate is green, and `rule:types/type-test` is `shipped` after a row-by-row walk of its table.
The last sweep's only red was a carried floor check — the pipeline operator's perf figure, staled by
this goal's own edit to `docs/reference/lang/30-expressions.md`, which is the file a `lang:` feature's
`impl_hash` is taken from. Re-measured and recorded; `python tools/dossier.py --only
lang:expressions/the-pipeline-operator --gate` is green.

**`self::`, `static::` and `parent::` in type position are `E0135`**, the question the table walk
left open. They are whole atoms, every member a `::` could reach is spelled with the owner's own
name, and the refusal recovers as `mixed` so one misspelt type is one error —
`rule:types/grammar`, `crates/nvs-syntax/src/parser/ty.rs:322`.

**One position that refusal does not reach is the statement head**, and it is not specific to this
code: `3.14 $b = 1.0;` reports `E0101` too. The playbook bullet is the trap; the next group is the
fix.

## Next group

**Stage 7 follow-on: a type refusal survives the declaration trial parse, one file set — the
statement head's trial parse and the type grammar it calls.**

- [ ] **Decide and land what makes a type-position diagnostic survive the trial parse.** Today
      `crates/nvs-syntax/src/parser/stmt.rs:1043` restores the checkpoint whenever
      `self.diags.len() > cp.diags_len`, which is right for `($a || $b) ? f() : g();` and wrong for
      `self::MAX $n = 7;` — the second lands on a `$variable`, which no expression statement can do.
      `rule:types/grammar` is the atom list the refusals belong to; the candidate signal is the
      variable, not the diagnostic count.
- [ ] **Pin it with a reject case over every refusal the type grammar reports**, beside
      `tests/conformance/reject/a-class-keyword-is-a-whole-type-and-never-carries-a-member.nvst`:
      `E0115` (`crates/nvs-syntax/src/parser/ty.rs:432`), `E0120`
      (`crates/nvs-syntax/src/parser/ty.rs:650`) and `E0135`
      (`crates/nvs-syntax/src/parser/ty.rs:322`) at the statement head, each reported where it is
      written rather than as `E0101`.

## Backlog

- A `lang:` feature's perf currency is its reference chapter's text, so any prose edit to
  `docs/reference/lang/*.md` stales every figure anchored there — `tools/dossier.py:1170`.
- `crates/nvs-syntax/src/parser/tests/ty.rs` carries no unit test for `E0135`; the conformance case
  is its only guard (`docs/rules/types.json`, `types/grammar` § guardedBy).
