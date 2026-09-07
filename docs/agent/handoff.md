# Handoff

## State

**Goal 14 answers six of the ten requests, and a declaration is now reached by its own name and
through a member.** `crates/nvs-lsp/src/definition.rs:135` is the one walk from a cursor to a
declaration — `nvs_hir::SymbolTable` for the declaring class, that file's kept statements for the
node — and the two requests read different fields off it: `definition` the name's span, `hover` the
`///` run above it. `crates/nvs-lsp/src/hover.rs` is the new request and holds only the Markdown.

**A member's span is in its class's body, never in `nvs_hir::MemberTable`.** That table records
which names a class declares and not where they were written. `ExprInfo::Call`, `CallableRef`,
`ClassRefCall`, `Property`, `StaticProperty` and `HookedProperty` each name the *declaring* class,
which is where the walk starts, so an inherited member answers the ancestor that wrote it.

**`nvs lsp-test tests/lsp/` reports `40 passed, 0 failed`**, against the goal's floor of 160.

**`nvs_syntax::DOC_MARKER` is public**, so no crate outside the parser spells `///` a second time.

**`semanticTokens/full` and hover's other two arms are what stage 6 has left.**

## Next group

**Stage 6: hover's two remaining arms** — one file set: `crates/nvs-lsp/src/hover.rs`,
`crates/nvs-lsp/src/definition.rs`, `crates/nvs-lsp/src/suite.rs`, `crates/nvs-lsp/src/render.rs`,
with `crates/nvs-stdlib/src/registry.rs` and `crates/nvs-types/src/ty.rs` read only.

- [ ] **`hover` over a `Core` member, out of the registry's signature row.**
      `crates/nvs-lsp/src/hover.rs:60` is where it stops today: a `Core` class has no
      `nvs_hir::Symbol`, so the shared walk answers nothing and this arm has to come before it.
      The row is `crates/nvs-stdlib/src/registry.rs:967`'s `CoreMethod`, whose card is
      `rule:core-api/reference-card`, and ADR 0088 § 5 is how it is spelled — now named in
      `[context.stage.6] adrs`, which is where it was missing.
- [ ] **`hover`'s declared-type arm, for a cursor that reached no declaration.**
      `crates/nvs-lsp/src/definition.rs:254`'s `_ => return None` is where an expression that has a
      type but names nothing falls out; `crates/nvs-types/src/ty.rs:521`'s `describe` spells a
      `TypeId`, and `crates/nvs-lsp/src/document.rs:298` is the interner it must be read against.
      ADR 0099 § 3's first `hover` cell, `rule:ide/the-rendering-has-one-home` for the spelling.
- [ ] **A `.lspt` case per arm, plus one where the two meet** — a `Core` call and a plain `int`
      expression under the same cursor rules. They land in `tests/lsp/hover/` beside the four this
      session froze, and the runner arm they go through is `crates/nvs-lsp/src/suite.rs:398`.
      `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.

## Backlog

- `semanticTokens/full` — stage 7, its own file set: a new module plus `render.rs`'s token spelling.
- `@see` rendered as a link — ADR 0099 § 3, blocked on resolving a member named in a comment;
  `crates/nvs-lsp/src/hover.rs:33` says what it would take.
- Hover on a declaration's **own** name answers nothing: the type table has no entry for a
  declaration node, so this needs the name-to-symbol step neither request has yet.
- A class constant under the cursor answers nothing — `nvs_types::ExprInfo` records `CoreConst` and
  nothing for a user one, so `definition`'s member arms cannot reach it.
- `documentHighlight`, inlay hints and the rest stay at M10 —
  `rule:ide/five-features-are-one-reference-index`.
