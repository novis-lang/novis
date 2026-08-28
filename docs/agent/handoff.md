# Handoff

## State

**M4, and Stage 00's first item is half landed.** ADR 0109's grammar half is on disk and green: a
`for` header's init clause is `nvs_syntax::ast::ForInit` — one ADR 0007 § 3.1 typed local declaration
(`int $i = 0`, `var $i = 0`) or the comma-separated expression list it has always been — decided by a
checkpointed trial parse in `parse_for_init`, the same backtracking `parse_stmt_maybe_local_decl`
already owns. `E0124` (a mixed clause) and `E0125` (two declarations) are the two refusals of § 3, each
raised once per header, with the declaration kept either way so the twelve-error cascade that ADR
exists to delete cannot come back through the recovery path. Four `.nvst` cases and all four of the
gate's `nvs-syntax` test names are written and passing.

`python tools/loop.py --goal-only` is still red at Stage 00 — its first check also wants item 50's
`a_php_shaped_enum_case_is_refused_naming_the_spelling_that_works`, and its conformance block names
three cases belonging to items 45-47. That is the worklist, not a breakage.

The blind spots the previous handoff recorded are unchanged and still worth knowing: `refusals.rs`
counts **arms**, not shapes, and `holes.py` recognizes a refusal by **phrasing**, so both undercount.
Items 45 and 49 are where that is fixed.

## Next group

**Item 44's second half — the corpus migrates to the header form.** ADR 0109 § *Consequences* is the
rule and the file set is `tests/conformance/`, `tests/differential/` and `examples/`: `grep -rln "for ("`
finds **84** files. The migration is mechanical but the exclusion is not, so read § *Consequences*
before starting — a counter read *after* its loop, or shared by two loops, stays declared above, and
moving it would change the case's meaning rather than its spelling. Every migrated case's expected
output is unchanged by construction (§ 2's function scope), so the suites already being green is the
whole review.

- [ ] **Migrate `tests/conformance/`**, the largest share of the 84. Take it directory by directory —
      `core/` is most of it — and run `./target/debug/nvs.exe test tests/conformance/<dir>` after each.
      ADR 0109 § *Consequences*, and `docs/adr/0109-a-for-header-declares-its-own-counter.md:146`.
- [ ] **Migrate `tests/differential/` and `examples/`.** Same rule; the differential tree's expectation
      is PHP's own output, which a spelling change cannot move.
- [ ] **Then item 50**, `a_php_shaped_enum_case_is_refused_naming_the_spelling_that_works` — the last
      name in Stage 00's first `[[check]]` block (`docs/agent/loop-goal.toml:241`), and the only one
      still unwritten now that ADR 0109's four are green. `python tools/holes.py --item 50` is the item.

Anchors for the work already landed, in case a migration turns up an edge:
`crates/nvs-syntax/src/parser/stmt.rs:329` (`parse_for_init`), `:414` (`try_parse_for_decl`),
`crates/nvs-syntax/src/ast.rs:1003` (`ForInit`), `crates/nvs-ir/src/lower/control.rs:452`
(`lower_for`).

## Backlog

- Item 45 — a user-declared class constant, checker and lowering halves (`docs/agent/loop-goal.md`).
- Item 46 — `new static()`'s type through two levels (ADR 0008).
- Item 47 — `instanceof` narrowing to an interface (`loop-goal.toml:255`).
- Item 48 — a `Core` class with no constructor refuses arguments.
- Item 49 — the shape table `crates/nvs-ir/tests/type_atoms.rs`, and re-deriving `refusals.rs`'s ceiling.
- ADR 0109 named `tests/conformance/diag/`, which does not exist; its *Verification* now names
  `tests/conformance/reject/` instead, which is where the tree keeps a refusal case.
