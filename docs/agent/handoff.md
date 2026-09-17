# Handoff

## State

**Goal `class-scoped-types`, stage 3 is complete: all five `reject` cases the stage check names are
on disk and green, and every refusal the stage's prose asks for is landed.**

- A name is a constant, a case or an alias, never two: one body declaring two of them under one name
  is `E0304` at the later declaration, collected in `crates/nvs-hir/src/aliases.rs`
  (`@check_name_collisions`) and reported by `AliasResolver::resolve`, which is the first call in
  that pass handed a `Diagnostics`. A collision an alias does not take part in is left to
  `crate::members`, so nothing is reported twice.
- `Sub::Name` where only an ancestor declares the alias is `E0405` naming the owner that does
  (`crates/nvs-types/src/lower.rs:@report_missing_member`, breadth-first over `extends`/`implements`
  through `Env::graph`), instead of the misleading "has no constant named".
- **A diagnostic identical to one already held is now dropped by `Diagnostics::report`**
  (`crates/nvs-diagnostics/src/diagnostic.rs:@identity`). A type written in a signature is lowered
  twice — once for the signature, once for the body that binds it — so *every* diagnostic from a
  signature's types arrived twice, `E0303` for an undefined class as much as the alias miss. The
  sink was the one place that fixes the class of them; the module doc on `Diagnostics` owns why, and
  `truncate` gives a withdrawn diagnostic its identity back so the parser's backtracking still
  re-reports.
- Stages 4 and 5 are what is left. Nothing is blocked.

## Next group

**Stage 4: the tooling sees the member, phase-gated** — one file set: `crates/nvs-fmt/`,
`crates/nvs-cli/src/meta.rs`, `crates/nvs-lsp/src/`.

- [ ] **The formatter lays a body's `type` out one way** — `crates/nvs-fmt/src/print.rs:137`
      (`print`). `nvs-fmt` is token-driven, not AST-driven, so the member may already format to
      itself; find out first with a case under `crates/nvs-fmt/tests/`, and only then reach for
      `space.rs`/`modifiers.rs`. `rule:tooling/fmt-novis-constructs`.
- [ ] **`nvs meta --json` emits a class-scoped alias in its owner's entry** —
      `crates/nvs-cli/src/meta.rs:803` (`user_alias_json`), under a `types` array, each card the
      shape that function already writes with the name spelled `Owner::Name` fully qualified.
      `rule:tooling/meta-json-takes-a-program`, goal stage 4.
- [ ] **Go-to-definition on `Owner::Name` in type position lands on the member** —
      `crates/nvs-lsp/src/definition.rs:283`. The other three LSP answers are the same stage and the
      same file set: `crates/nvs-lsp/src/symbols.rs:102` nests it under its owner,
      `crates/nvs-lsp/src/semantic.rs:677` colours it as a type,
      `crates/nvs-lsp/src/completion.rs:493` offers it after `Owner::`.

## Backlog

- Stage 5 — the rule fragments and the one new decision record (goal stage 5 owns the numbers).
- Signature types are still lowered twice; the duplicate *report* is gone but the duplicate *work*
  is not (`crates/nvs-types/src/signatures.rs:1044` and `crates/nvs-types/src/check.rs:670`).
- `rule:types/type-alias` still reads "never inside a class, and never inside a body" — stage 5's
  fragment work, not a stray edit.
