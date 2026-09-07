# Handoff

## State

**Goal 14, stage 8 is closed.** `nvs/redactions` answers: `crates/nvs-lsp/src/redactions.rs` is the
walk, `server.rs` dispatches the method and `suite.rs` runs a `redactions` case. That runner's
`answer` match now has no wildcard — every request in the closed set has an arm, so a tenth is a
build error rather than a case that passes by being skipped.

**The walk is a projection of `nvs_syntax::walk::of_stmts`, not a second match over the AST.** It
needs a literal's span and an interpolation slot's span and no name span, so it takes the one
exhaustive in-crate traversal, and a production landing later cannot quietly stop being concealed —
which is the opposite trade from `semantic.rs`, whose module doc explains why it pays the wildcard.

**A literal is attributed through the binding it is written into**, which is ADR 0101 § 1's fail
direction asked everywhere rather than only on failure: a `secret` `LocalDecl`'s initializer, an
assignment whose target carries `secret`, and an interpolation slot whose own expression does. The
value is followed only through `()`, a ternary's branches, `.` and `??`. A parameter's default and a
property *declaration*'s default are the two positions it does not reach, both named in that module
doc and both pinned by `tests/lsp/redactions/`.

`nvs lsp-test tests/lsp/` reports `79 passed, 0 failed` against the goal's floor of 160. The earliest
red acceptance check is now stage 9's code actions, which is the group below; stage 4's case floor is
fed by every slice after it.

## Next group

**Stage 9: two code actions, and the boundary** — one file set: a new
`crates/nvs-lsp/src/actions.rs`, with `crates/nvs-lsp/src/case.rs`, `crates/nvs-lsp/src/render.rs`,
`crates/nvs-lsp/src/server.rs` and `crates/nvs-lsp/src/suite.rs` edited, and
`crates/nvs-diagnostics/src/diagnostic.rs:112`'s `Suggestion` read for what a fix already carries.

- [ ] **A `codeAction` case can be written** — `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`,
      `rule:ide/the-rendering-has-one-home`. A variant beside `crates/nvs-lsp/src/case.rs:75`, in
      `Request::ALL` and `Request::name`, and `takes_cursor` true — the request is asked over a
      range, so the case's `<|>` is what names it. The answer variant goes beside
      `crates/nvs-lsp/src/render.rs:129` and its renderer beside `crates/nvs-lsp/src/render.rs:409`.
- [ ] **A code action is a suggestion the diagnostic already carried** —
      `rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`. The new module reads the
      analysis's diagnostics through `crates/nvs-lsp/src/diagnostics.rs:195`, which already says a
      `nvs_diagnostics::Suggestion` is a code action, and offers nothing a checker would have to
      compute (`rule:ide/a-code-action-writes-only-what-is-already-determined`). Register it at
      `crates/nvs-lsp/src/lib.rs:106` and hand the server its arm beside
      `crates/nvs-lsp/src/server.rs:163`.
- [ ] **The four named claims are frozen** — the same rules. `a_code_action_comes_from_a_suggestion_the_diagnostic_already_carried`,
      `the_casing_diagnostic_carries_its_replacement`, `the_legacy_cast_diagnostic_carries_its_replacement`
      and `a_fix_the_checker_would_have_to_compute_is_offered_by_nothing`, in
      `crates/nvs-lsp/tests/redactions.rs:20`'s shape — an open buffer over a path that need not
      exist — plus `.lspt` cases under `tests/lsp/actions/`.

## Backlog

- The `.lspt` corpus is 79 of the floor of 160; every remaining request slice feeds it (`docs/agent/loop-goal.toml:5252`).
- `nvs lsp-test --coverage` and `every_request_answers_every_construct` are unwritten, and they are the goal's real gate (`docs/agent/loop-goal.toml:5311`).
- A property *declaration* carries no qualifier to ask, so neither `semantic` nor `redactions` reaches its default; closing it is a `nvs-types` change.
- Stage 10's latency guard and stage 11's reference chapter are untouched.
