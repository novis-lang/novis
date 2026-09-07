# Handoff

## State

**Goal 13 stage 2 is unchanged and green** — `|>` lexes, parses between `instanceof` and unary, and
substitutes its one `$_` at parse time, with the cases under `tests/conformance/reject/pipeline/`,
`tests/conformance/lang/` and `examples/pipeline.nvs` pinning it.

**Three of ADR 0124's four PHP 8.6 refusals now land, all in the parser** where the other rejected-PHP
spellings live. `let` and `is` are keywords, so either written as a name is `E0247`, whose help names
the living spelling (`var` declares, `instanceof` tests, `as` converts); `is` written between two
operands is refused and then parsed as the `instanceof` it names, so one expression carries one
diagnostic. `E0248` refuses a value on a constructor's `return` and `E0249` refuses a default on a
`readonly` property. `Parser::in_callable_body` is the machinery the first of those needed and the
`finally` refusal will reuse: it parks `in_constructor` at every body a `return` can belong to — a
closure, a property hook, a method declared inside the constructor — so the flag answers "does this
`return` leave the constructor", not "is a constructor anywhere above this".

**What stage 3 still owes:** `rule:php-migration/no-return-leaves-a-finally`, and the `.nvst` cases
under `tests/conformance/reject/php86/`, a directory that does not exist yet — the stage's second
check runs it, so that check stays red until the cases are written. Nothing is blocked.

## Next group

**The `finally` refusal and stage 3's corpus, one file set:** `crates/nvs-syntax/src/parser/stmt.rs`,
`crates/nvs-syntax/src/parser/mod.rs`, `crates/nvs-diagnostics/src/lib.rs`,
`tests/conformance/reject/php86/`. The rejected-PHP band's next free number is `E0250`.

- [ ] **A `return` never leaves a `finally`**, nor a `break` or `continue` whose target lies outside
      it — `rule:php-migration/no-return-leaves-a-finally`. The `finally` block is parsed at
      `crates/nvs-syntax/src/parser/stmt.rs:680`; park and set the state with
      `crates/nvs-syntax/src/parser/mod.rs:625`'s `in_callable_body`, which already does exactly this
      job for `in_constructor` and is the shape to copy. A `break`/`continue` needs the count of
      loops and `switch`es opened *inside* the block, so `crates/nvs-syntax/src/parser/stmt.rs:305`,
      `:586` and their three siblings each raise it around their body, and
      `crates/nvs-syntax/src/parser/stmt.rs:649` reads it against the written level.
- [ ] **The three refusals as `.nvst` cases** under `tests/conformance/reject/php86/`, which the
      stage's `nvs-suite` check runs — a `return` in a `finally`, a constructor returning a value and
      a `readonly` default. `--EXPECTF-ERROR--` reproduces the diagnostic's own indentation;
      `tests/conformance/reject/readonly-is-written-once.nvst:1` is the nearest neighbour to copy the
      shape from. Never `--ORACLE--` under `tests/conformance/`.

## Backlog

- The `php-migration` and `expressions` rules this goal implements still read `status: designed` in
  `docs/rules/php-migration.json` and `docs/rules/expressions.json`; flipping them wants
  `python tools/rules.py --render` in the same commit, and `guardedBy` wants the `.nvst` paths above.
- `crates/nvs-syntax/src/token.rs:317` says every `Keyword` variant's `name()` gives its one spelling;
  there is no such method on the enum.
- Whether `nvs convert` applies the two mechanical rewrites ADR 0124 names — dropping a constructor's
  returned value, and moving a `readonly` default — is unwritten; the rules say it does.
