# Handoff

## State

**M4's frontier is Stage 5's item 33, and only its property half is left.** ADR 0092's record
model and plaintext rendering are the leaf crate **`nvs-render`**; `Core\Debug::dump`/`render`
are registered `Core` members; and the redaction row's **call-site** half now refuses a
`secret` argument at either member (`E0724`).

- **Redaction is two halves of one rule** (ADR 0092 § 5). The call-site one is
  `nvs_types::expr::quals::reject_secret_debug_argument`
  (`crates/nvs-types/src/expr/quals.rs:263`), called from `infer_static_call`
  (`crates/nvs-types/src/expr/calls.rs:188`). The property one — a `secret`-typed property
  rendering as `Redacted` — is untouched and needs a descriptor bit the runtime does not carry.
- **It is a call-site rule because both members declare `mixed`**, which a `secret string`
  satisfies; the checker is the last point the qualifier exists at. Same mechanism split ADR
  0033 § 4 already makes for `Core\Log::write`.
- **`nvs-render` depends on `nvs-syntax` and that edge is temporary** — ADR 0087's bidi
  predicate, called rather than restated. It inverts (a **move** of `nvs_syntax::bidi` down)
  the moment `nvs-runtime` or `nvs-diagnostics` becomes a dependent. That crate's module doc
  § *Where this sits* is the one home for it.
- **`Ctx` has a second sink.** `write_diagnostic` (`crates/nvs-runtime/src/ctx.rs:960`) writes
  to `OutputSink::Stderr` by default and is not routed through the capture stack.

## Next group

**Finishing item 33: the property half, then the caps case.** The file set is
`crates/nvs-types/src/layout.rs`, `crates/nvs-stdlib/src/instance.rs`,
`crates/nvs-runtime/src/object.rs` and `crates/nvs-stdlib/src/debug.rs` — the vertical a
per-field tag has to run down, `ClassDesc::renderer`/`ClassDesc::unwind` being the two
precedents for putting a compile-time answer on the descriptor.

- [ ] **A `secret`-typed property renders as `Redacted`** — ADR 0033 § 4's debug-dump bullet,
      ADR 0092 § 5's redaction row. The walk is
      `nvs_stdlib::debug` (`crates/nvs-stdlib/src/debug.rs:126`) and the node already exists
      (`nvs_render::Node::Redacted`); what is missing is the *declared* type reaching the
      walk, which only a descriptor bit can carry — `ClassDesc::renderer` (set in
      `nvs_stdlib::instance`) is the shape to copy, one bit per slot rather than one per class.
      `nvs_types::expr::quals::is_secret` (`crates/nvs-types/src/expr/quals.rs:59`) is the
      predicate that decides it at the one end that knows.
- [ ] **The elision and depth caps get their own case** — ADR 0092 § 5's fourth M4 bullet. Over
      landed work, so it is a `.nvst` and nothing else: a cut is an `Elision` node built in
      `nvs-render`, so the case asserts what the plaintext view prints at the boundary and one
      past it, both sides named together.

## Backlog

- A `require` whose path is not a string literal runs nothing at all, silently, in both forms —
  `nvs_hir::requires`' own known gap.
- `Core\Secret::reveal` does not exist in `nvs_stdlib::registry`, so `E0724`'s help names a
  spelling no program can write yet (ADR 0033 § 4, M6/M8).
- The `Throwable` producer of ADR 0092 § 6 waits on the crate edge above `nvs-runtime`.
- An enum case dumps as its backing integer — stated at `nvs_stdlib::debug`, ADR 0010 § 5.
- ADR 0024 § 4's sink list and ADR 0033's `Core\Log` inspection wait on M7/M8 `Core` classes.
