# Handoff

## State

**Goal 12 — the resilient tree — has stages 2 and 3 landed and green.** Recovery is explicit:
`MemberName::Missing(Span)` is what `parse_member_name` yields where nothing was written, and
`ExprKind::Error(Span)` carries what it stood in for, so no consumer infers recovery from a span's
width (`rule:ide/recovery-is-explicit`). The `SyntaxIndex` itself landed in an earlier session; what
stage 3 owed and now has is the four tests its acceptance check names, including the strict/lossless
agreement test that is the regression this goal is closest to.

**One residue, deliberate:** `ExprKind::ClassConstAccess` carries a bare `Span` for its name, so
`Foo::` at the caret still carries a name nobody wrote. The `::` path routes `Missing` into the
same arm `Ident` takes rather than reporting an invented second diagnostic; the comment at that site
says so, and the Backlog carries the fix.

**Stage 0 is empty and stays empty**, unchanged: the four M4 language holes are closed and `bool as
int` is `E0708` under `rule:types/conversion`, not a hole.

## Next group

**Stage 4, the prefix sweep** — one new file, `crates/nvs-syntax/tests/prefixes.rs`, modelled on the
corpus walk at `crates/nvs-syntax/tests/lossless.rs:136`, which already lexes every file in
`examples/`, `tests/` and the vendored `php-src` checkout. All three slices are that one file plus
the two entry points they call, so they share a file set and should go together.

- [ ] **`every_prefix_of_every_example_parses_without_panicking`** — cut every `examples/*.nvs` at
      every token boundary and parse each prefix with `crates/nvs-syntax/src/parser/mod.rs:771`.
      `rule:ide/the-tree-survives-a-syntax-error`; M4B's acceptance paragraph names the sweep.
- [ ] **`every_prefix_answers_a_syntax_index_lookup_at_its_end`** — `at()` at the final offset of
      each prefix answers rather than panicking: `crates/nvs-syntax/src/index.rs:136`, remembering
      that a caret at a node's end is between nodes (that file's second decision).
      `rule:ide/the-index-answers-the-cursor`
- [ ] **`an_incomplete_prefix_still_reports_a_diagnostic`** — a prefix that is genuinely incomplete
      reports one, which is what separates recovery from silence; `crates/nvs-syntax/src/parser/mod.rs:771`
      is the entry point, and a prefix ending at a statement boundary is the case to exclude.
      `rule:ide/recovery-is-explicit`

## Backlog

- `ClassConstAccess`'s name is a bare `Span`, so the `::` path cannot say a name was invented —
  `crates/nvs-syntax/src/ast.rs`, and every consumer of that variant. `rule:ide/recovery-is-explicit`
- Stage 5, position arithmetic in `nvs-diagnostics` — `docs/agent/loop-goal.toml`.
- Stage 6, one section lexer for `.nvst` and `.lspt` in `nvs-test` — `docs/agent/loop-goal.toml`.
- Stage 7, `nvs ast --json --resilient` and its frozen schema — `docs/agent/loop-goal.toml`.
