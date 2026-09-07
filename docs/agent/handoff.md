# Handoff

## State

**Goal 13's stage 2 is whole and green.** `|>` lexes as one token, parses between `instanceof` and
unary, and substitutes its one `$_` at parse time; the four refusal cases under
`tests/conformance/reject/pipeline/`, the substitution-identity case in `tests/conformance/lang/` and
`examples/pipeline.nvs` now pin all of it, so every stage-2 check in `docs/agent/loop-goal.toml`
passes — including the `exact` check whose missing fixture the driver had been reporting.

**The example turned up one true consequence of substitution:** `nvs ast --json`'s tree for a pipeline
is not in source order, since the left side becomes an argument of a call written after it, so
`crates/nvs-cli/tests/ast.rs`'s schema walk now takes an `in_source_order` flag and asserts sibling
order and containment only for the files that have one. A position-to-node lookup — the LSP goal 14
opens — cannot assume either over a pipeline; nothing else in the tree changes.

**Stage 3 — ADR 0124's four PHP 8.6 refusals — is untouched.** `let` and `is` still parse as ordinary
identifiers, a `return` still leaves a `finally`, a constructor may still return a value and a
`readonly` property may still declare a default. Stage 1's floor is untouched. Nothing is blocked.

## Next group

**Stage 3's refusals — the parser side and its corpus, one file set:** `crates/nvs-syntax/src/token.rs`,
`crates/nvs-syntax/src/parser/stmt.rs`, `crates/nvs-syntax/src/parser/decl.rs`,
`crates/nvs-diagnostics/src/lib.rs`, `tests/conformance/reject/php86/`. Each refusal is a diagnostic
plus the case that shows it; the band is the decision's own call — the parser band's next free number
is `E0132` and the rejected-PHP band's is `E0247`, both at `crates/nvs-diagnostics/src/lib.rs:235`.

- [ ] **`let` and `is` are reserved spellings**, `var` declaring and `instanceof` testing instead —
      `rule:php-migration/let-and-is-are-reserved`. The keyword table is the match at
      `crates/nvs-syntax/src/token.rs:468`, and `docs/agent/loop-goal.toml:5067` names the two tests
      `let_is_a_reserved_spelling` and `is_is_a_reserved_spelling`, which belong beside their
      neighbours in `crates/nvs-syntax/src/parser/tests/expr.rs:1209`.
- [ ] **A `return` never leaves a `finally`**, nor a `break` or `continue` whose target lies outside
      it — `rule:php-migration/no-return-leaves-a-finally`, at
      `crates/nvs-syntax/src/parser/stmt.rs:653`.
- [ ] **A constructor's `return` carries no value and a `readonly` property declares no default** —
      `rule:php-migration/a-constructor-return-carries-no-value` and
      `rule:php-migration/a-readonly-property-declares-no-default`, both reached from
      `crates/nvs-syntax/src/parser/decl.rs:649`.
- [ ] **The three refusals as `.nvst` cases** under `tests/conformance/reject/php86/`, which
      `docs/agent/loop-goal.toml:5076`'s `nvs-suite` check names — `--EXPECTF-ERROR--`, never
      `--ORACLE--`, and a case reaches a `Core` class only as `Core\Str::…`.

## Backlog

- Stage 4 — the reference chapter that follows the operator (`docs/agent/loop-goal.toml` stage 4).
- `?|>` stays deferred with its trigger named — ADR 0098 § *Revisiting*, not this goal's work.
- Partial function application stays non-adopted — `rule:php-migration/no-partial-application`.
