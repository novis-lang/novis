# Handoff

## State

**Goal 13 — the pipeline operator and the PHP 8.6 refusals — has just started; nothing of it has landed
yet.** Goal 12's whole list is this goal's Stage 1 floor. Both designs are written and accepted
(ADR 0098, ADR 0124); this goal implements them and reopens neither.

**One thing is already known and is stage 0.** ADR 0098's table assigns `E0124`, `E0125` and `E0126`, and
all three were allocated to other diagnostics after it was written — ADR 0109's two `for`-header codes and
ADR 0119's catch-arm code, in `crates/nvs-diagnostics/src/lib.rs`. The registry is the allocator, so the
pipeline's codes move to the next free numbers in the parser band and ADR 0098's body is folded to match.
Do that before writing a single case.

## Next group

**Stage 0 and stage 2 together: the renumbering, then the operator** — one file set:
`crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-syntax/src/token.rs`,
`crates/nvs-syntax/src/parser/expr.rs`, and `docs/adr/0098-pipeline-operator-is-a-hole-substituted-at-parse-time.md`.

- [ ] **The three codes, reassigned**, in the registry and folded into ADR 0098's table in its own body.
      No overlay note and no new ADR number; the retired-code rule means the old numbers are not shuffled.
- [ ] **The `|>` token, the `$_` hole, § 2's precedence, and the substitution** in the parser. The
      acceptance list's first test is the design's whole claim: the AST is the nested spelling's AST.
- [ ] **The three refusals** and their `tests/conformance/reject/pipeline/` cases, including the PHP 8.5
      callable shape whose help line names the difference.

## Backlog

- Stage 3 (`let`/`is` reserved, and the three ADR 0124 checks) shares `token.rs` and `parser/decl.rs` —
  a session that lands the group above with headroom takes the reserved spellings, which are two lines
  and two tests.
- Stage 4 (the reference heading and its four proofs) is prose plus three examples and one bench, and
  it is the cheapest session to start cold.
- **Off path:** anything in `crates/nvs-lsp` or `editors/` — goals 14 and 15. ADR 0098's deferred `?|>`
  and ADR 0124's non-adoptions stay closed.
- When this goal's last check goes green the chain advances to goal 14 — `crates/nvs-lsp`, the server
  behind `nvs lsp`.
