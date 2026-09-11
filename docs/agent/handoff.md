# Handoff

## State

**Goal `markup-literal`, stage 3 is landed: `` html`…` `` parses to `ExprKind::Markup` and types as
`Core\Html\Markup`.** Stage 1 (goal `finish-response`'s list) is the untouched floor; stage 2 (the lexer
and the AST node) is on disk from the previous session.

`Parser::parse_markup_literal` (`crates/nvs-syntax/src/parser/expr.rs:2523`) runs `parse_string_body`
with `TokenKind::MarkupClose` and deliberately skips `collapse_string_parts`: the node, not the part
count, is what says `Core\Html\Markup` (`rule:core-classes/html-literal`).

`infer_markup_literal` (`crates/nvs-types/src/expr/literals.rs:480`) answers the carrier class whatever
the body holds, and asks a hole three questions that are *not* an interpolation's three — a `Markup`
hole is spliced raw so `reject_carrier_as_text` does not run, a `tainted` hole is admitted and does not
spread, a `secret` hole is `E_SECRET_OUTPUT` at the hole's own span. All four stage-3 acceptance tests
pass, in `crates/nvs-types/tests/markup_literal.rs`.

**Nothing lowers `ExprKind::Markup` yet.** Every `ExprKind` match below the checker still has a
catch-all, so a program using the literal compiles and then silently lowers nothing — the next group is
where the IR arms get written deliberately.

## Next group

**Stage 4: the lowering, and the two shapes that pay nothing** — one file set: `crates/nvs-ir/src/lower/`
and `crates/nvs-codegen/tests/`.

- [ ] **The value-position lowering** — `crates/nvs-ir/src/lower/expr.rs:131`, beside the
      `ExprKind::Interpolated` arm whose part-joining helper is
      `crates/nvs-ir/src/lower/expr.rs:1998`: a `Markup` in value position is one carrier holding the
      joined bytes, each hole through `Core\Html::escape` and each segment raw
      (`rule:core-classes/html-literal` § *What it costs to run*). `a_markup_literal_assigned_to_a_local_does_build_one`
      and `a_markup_literal_returned_from_a_function_does_build_one` are the two checks.
- [ ] **The sink-position lowering** — `crates/nvs-ir/src/lower/control.rs:2169`, the `echo` statement's
      own `Interpolated` arm: a literal written straight to the sink lowers to a run of writes —
      segment, escaped hole, segment — and materialises no carrier, because a value born and consumed at
      one sink is unobservable. Check: `a_markup_literal_echoed_lowers_to_writes_and_builds_no_markup`.
- [ ] **The hole-free fold** — the same `crates/nvs-ir/src/lower/expr.rs:131` arm: a literal with no
      holes is constant-pool data, as a duration literal is (`rule:types/duration-literal`), so it
      allocates nothing per execution. Checks: `a_hole_free_markup_literal_folds_to_one_constant` and
      `a_hole_free_markup_literal_in_a_loop_allocates_once_for_the_whole_loop`, both `-p nvs-codegen`.

## Backlog

- Stage 5 `Core\Html::join` — `crates/nvs-stdlib/src/html.rs`, three `-p nvs-stdlib` tests.
- Stage 5's seven `.nvst` cases — `docs/agent/loop-goal.toml`'s `nvs-suite` check lists them by path.
- `ExprKind::Markup` reaches `nvs-hir`, `nvs-lsp` and `nvs-diagnostics` walks only through a catch-all
  today; `crates/nvs-hir/src/members.rs:910` and `crates/nvs-lsp/src/semantic.rs:800` are the two arms
  that will want it once the literal is used in earnest.
- The pack's `[context] modules` named only `crates/nvs-types/src/core_lib.rs` for nvs-types, so
  `expr/literals.rs`, `expr/mod.rs` and `tests/common/mod.rs` were all found by hand; the driver sweeps
  `modules` from committed paths, so this closes itself.
