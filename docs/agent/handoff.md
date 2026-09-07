# Handoff

## State

**Goal 13's stage 0 and the parser half of stage 2 are on disk and green.** `|>` lexes as one token,
parses between `instanceof` and unary, and substitutes its one `$_` at parse time; there is no `ExprKind`
for the operator and none for the hole, so nothing below `nvs-syntax` changed or had to.

The three codes are `E0129`, `E0130` and `E0131` — the next free parser-band numbers, since the numbers
ADR 0098 wrote had been allocated to `for`-header and catch-arm diagnostics in the meantime. `E0124`–
`E0126` were left with their owners (the registry never reuses or shuffles a number), and § 4's table,
`rule:expressions/pipeline-hole-once`, `docs/plan/m1.md` item 5 and ADR 0124 § 5's cross-reference all
carry the new ones.

Nothing is blocked. Stage 1's floor and stage 3's PHP 8.6 refusals are untouched.

## Next group

**Stage 2's proofs — the corpus and the example — one file set:** `tests/conformance/reject/pipeline/`,
`tests/conformance/lang/`, `examples/pipeline.nvs`. The parser side they pin is already landed, so each
of these is authored against a binary that already answers; build `cargo build -p nvs-cli` and run
`target/debug/nvs.exe` rather than the release one.

- [ ] **The three refusals' `.nvst` cases** under `tests/conformance/reject/pipeline/`, which
      `docs/agent/loop-goal.toml:5037`'s `nvs-suite` check names — `rule:expressions/pipeline-hole-once`.
      Four cases, because `E0129` has two forms: a right side with no hole, the PHP callable shape
      (whose extra note is at `crates/nvs-syntax/src/parser/expr.rs:569`), two holes, and a `$_` written
      outside any `|>` (`crates/nvs-syntax/src/parser/expr.rs:608`). The codes are
      `crates/nvs-diagnostics/src/lib.rs:224`. `--EXPECTF-ERROR--`, never `--ORACLE--`.
- [ ] **The substitution identity, as a conformance case** in `tests/conformance/lang/` —
      `rule:expressions/pipeline-substitution`. A `|>` chain and its nested spelling printing the same
      output over the four right-side shapes the unit test already compares as trees
      (`crates/nvs-syntax/src/parser/tests/expr.rs:1209`), which is ADR 0098's own M1 verification line.
- [ ] **`examples/pipeline.nvs`**, whose two frozen lines are `docs/agent/loop-goal.toml:5043`'s `want`:
      `a chain reads left to right` and `the nested spelling answers the same`.
      `rule:testing/examples-live-in-the-repository`.

## Backlog

- Stage 1's floor — goal 12's whole list — is still the goal's own precondition; nothing here touched it.
- Stage 3, the four PHP 8.6 refusals (`rule:php-migration/a-deprecation-is-a-refusal`), is a different
  file set: `crates/nvs-syntax/src/parser/decl.rs` and `stmt.rs`, plus `let`/`is` in `token.rs`.
- `?|>` stays deferred with its trigger named in ADR 0098 *Revisiting*; the corpus written now is the
  evidence that decides it.
- `docs/novis.md`'s code index carries no `E0129`–`E0131` row yet; it filters to `shipped`, and
  `rule:expressions/pipeline-substitution` is still `designed` until stage 2's proofs land.
