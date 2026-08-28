# Handoff

## State

**M4's frontier is Stage 5's item 33, and its larger half landed.** ADR 0092's record model and
plaintext rendering exist as the new leaf crate **`nvs-render`**, and `Core\Debug::dump`/`render`
are registered `Core` members with a byte-for-byte conformance case.

- **The model is `nvs-render`, not a member module.** ADR 0092 § 1 puts the model and all three
  renderings in one crate both the runtime and the front end depend on; the handoff's stated file
  set predated that section being read. § 1's own text is corrected to say the crate exists.
- **§ 5 is structural, not a convention.** `nvs_render::Rendered::new`
  (`crates/nvs-render/src/lib.rs:148`) is the *only* way to put text in the model and it
  substitutes, so no producer can forget to. A cut is a node (`Elision`), a cycle is a node.
- **`nvs-render` depends on `nvs-syntax` and that edge is temporary** — ADR 0087's bidi predicate,
  called rather than restated. It inverts (a **move** of `nvs_syntax::bidi` down) the moment
  `nvs-runtime` or `nvs-diagnostics` becomes a dependent. That crate's module doc § *Where this
  sits* is the one home for it.
- **`Ctx` has a second sink.** `write_diagnostic` (`crates/nvs-runtime/src/ctx.rs:960`) writes to
  `OutputSink::Stderr` by default and is not routed through the capture stack.
- **`orient.py`'s pack did not print ADR 0092** — the item's own line said so and named the fix.
  Its `[context] adrs` now wants `0092` §§ 1, 3, 4, 5, and `0086` § 1 for the substitution table.

## Next group

**Finishing item 33: redaction, then the two cases it owes.** The file set is
`crates/nvs-types/src/expr/quals.rs`, `crates/nvs-ir/src/lower/mod.rs`,
`crates/nvs-codegen/src/lib.rs`, `crates/nvs-runtime/src/object.rs` and
`crates/nvs-stdlib/src/debug.rs` — the same vertical `ClassDesc::field_tags` already runs down.

- [ ] **A `secret` value at a `Core\Debug::dump` call site is refused by `nvs check`** — ADR 0033
      § 4, ADR 0092 § 5's redaction row. The cheap half and independent of the descriptor:
      `nvs_types::expr::quals::is_secret` (`crates/nvs-types/src/expr/quals.rs:59`) is the
      predicate, and the two refusals beside it are the shape to copy —
      `reject_secret_markup_conversion` (`:145`) and `reject_secret_throwable_message` (`:238`),
      the second of which is already called from a *call site* at
      `crates/nvs-types/src/expr/calls.rs:257`. Takes the band's next free code (`E0724`).
- [ ] **A `secret`-typed property renders as `Redacted`** — ADR 0033 § 4, whose own *Verification*
      defers this to M4 by name. One bit per slot down the same vertical `field_tags` runs:
      `nvs_types` knows the declared type, `nvs_ir::ir::Class::field_reprs`
      (`crates/nvs-ir/src/ir.rs:96`) is built by `field_reprs`
      (`crates/nvs-ir/src/lower/mod.rs:391`, called at `:636`) and erases the qualifier, so this
      is a **second** list beside it; `nvs-codegen` copies it at `crates/nvs-codegen/src/lib.rs:530`
      next to `set_field_tags`; `ClassDesc` (`crates/nvs-runtime/src/object.rs:206`) gains the
      accessor beside `field_name` (`:490`); and `nvs_stdlib::debug::object_body` reads it.
      Deletes gap 1 of `crates/nvs-stdlib/src/debug.rs`.
- [ ] **The elision and depth caps get their own case** — ADR 0092 § 5's fourth M4 bullet, which
      the standing case does not reach: a value past `Caps::depth`/`entries`/`text`
      (`crates/nvs-render/src/lib.rs:290`) renders an `Elided` node **naming what was cut**. Build
      the subject from a loop rather than a literal, since the caps are 8/100/1024.

## Backlog

- ADR 0092 § 6's `Throwable` producer (a record at `Error`, frames as Sequence-of-Object nodes) —
  needs the `nvs-syntax` → `nvs-render` edge inverted first; `crates/nvs-render/src/lib.rs`
  § *Where this sits*.
- An enum case dumps as its backing integer — `crates/nvs-stdlib/src/debug.rs` gap 2; ADR 0010 § 5
  spends no tag on hiding one.
- The JSON and HTML renderings, and `[debug] inline` — ADR 0092 §§ 3–4, M7.
- `Core\Log::write` and `Log\Level` as a Novis enum — ADR 0092 §§ 2 and 6, M8.
- `Cli::colorDepth`-driven colour for the plaintext rendering — ADR 0092 § 3, `nvs_stdlib::cli`
  gap 2, M8.
- Then Stage 6 opens (items 37–40, `crates/nvs-types/src/locals.rs`).
