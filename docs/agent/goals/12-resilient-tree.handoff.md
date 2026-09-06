# Handoff

## State

**Goal 12 — the resilient tree — has just started; nothing of it has landed yet.** Goal 11's whole list
is this goal's Stage 1 floor, and goal 11 already built the trivia layer and `Parsed { stmts, trivia }`,
so the tree work left is the index and explicit recovery rather than the whole of ADR 0099 § 1.

**Stage 0 is empty and stays empty.** The four M4 language holes M4B was staged with are closed, and
`bool as int` is not the fourth one — it is `E0708`, a decision of `rule:types/conversion`. A session that finds a
playbook bullet claiming otherwise has found a stale bullet, not a hole.

## Next group

**Stage 2 and stage 3 together: recovery, then the index** — one file set:
`crates/nvs-syntax/src/ast.rs`, `crates/nvs-syntax/src/parser/expr.rs`,
`crates/nvs-syntax/src/parser/mod.rs`. Both edit the same three files, and the index's ancestor path is
what makes a recovery node reachable, so splitting them across sessions pays the parser's orientation
cost twice.

- [ ] **`MemberName::Missing(Span)`** beside `MemberName::Ident` in `ast.rs`, produced where the
      synthesized name is made today — `python tools/peek.py --locate` finds the site in
      `parser/expr.rs`. Plus a span on `ExprKind::Error` naming what it stood in for.
- [ ] **`SyntaxIndex` in `Parsed`**, filled by one walk, answering `at(offset) -> NodePath` — innermost
      node plus its ancestors outward, in order. `parse_file` stays a thin wrapper and no call site
      changes; the named test for that is in the TOML's stage 3 check.

## Backlog

- Stage 4 (the prefix sweep and the `fuzz/` target) shares `parser/mod.rs` with stage 3 — a session that
  lands the group above with headroom takes it next. The fuzz target itself is a CI concern: it joins
  `lex` and `parse` in the `fuzz-smoke` job, and no loop check runs it.
- Stage 5 (`utf16_col`/`offset_of`) is `nvs-diagnostics` alone and shares nothing with the parser, which
  makes it the cleanest session to start cold.
- Stages 6 and 7 (`nvs-test`'s section lexer; `nvs ast --json`) are each one file set of their own.
- **Off path:** `nvs lsp-test`, the `.lspt` format itself, and anything under `editors/`. Those are
  goals 14 and 15. `## Backlog` and move on.
- When this goal's last check goes green the chain advances to goal 13 — `rule:expressions/pipeline-substitution`'s pipeline operator
  and ADR 0124's PHP 8.6 refusals, the two front-end items the editor's grammar must colour correctly.
