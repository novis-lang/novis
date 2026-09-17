# Handoff

## State

**Goal `class-scoped-types`, stage 4 is landed: the formatter, `nvs meta --json` and all four LSP
requests see the member.** Stage 5 — the rule and the record — is the only stage left, and nothing
is blocked.

- **The LSP had no way to say what type a cursor is in, and now has one.**
  `crates/nvs-lsp/src/definition.rs:@written_type_at` walks the positions a *declaration* writes a
  type at, because `nvs_syntax::walk` builds no node for one; the module doc lists them, and a type
  written inside an expression (`as`, `is`, a closure literal) is deliberately not among them.
- **`Owner::Name` in type position is answered by `@type_member_at`**, which `named_at` now ends in:
  a new `Target::TypeAlias` when the owner declares one, the `Target::Constant` an enum case or a
  class constant already was otherwise, and the owner itself when the caret is in the owner half.
- **What a written name means at a cursor has one home**: `resolved_name`, `namespace_at` and
  `imports_of` are `definition.rs`'s, and `completion.rs` reads them from there.
- **Completion after `Owner::` in type position** offers the owner's aliases, class constants and
  enum cases and nothing that cannot stand in a type. It is read off the source for the same reason
  the walk exists, so it answers in a document being typed into as well as one that parses.

## Next group

**Stage 5: the rule and the record** — one file set: `docs/rules/types.json`,
`docs/rules/types/type-alias.md`, `docs/decisions/`.

- [ ] **One new record and no other number** — `docs/rules/types/type-alias.md:1` is what its
      `changes:` block modifies, and the number is the next free one at the moment it is created
      (`docs/decisions/0189.md:1` is the highest on disk now; re-derive it rather than trusting this
      line). It argues the three calls in the goal's § *Standing decisions* — no visibility, lexical
      scope, no third spelling — and states what the feature spends: nothing per request, one table
      entry per declaration at compile time.
- [ ] **The fragments** — `docs/rules/types/type-alias.md:1-2` still says an alias is declared "never
      inside a class", which stage 0 named as the sentence stage 2 made wrong; its first paragraph
      names both declaration sites instead, and a new `types/class-scoped-alias` opens on a sentence
      that stands alone in `ground-rules.md`. Both list the stage 2–3 conformance cases in
      `guardedBy`. `docs/rules/types.json:1` is the chapter index they are registered in.
- [ ] **The gates** — `python tools/rules.py --render` rewrites the generated chapter in the same
      commit, `python tools/rules.py --check` is green, and `python tools/verify.py --doc` runs before
      the DONE claim. `docs/rules/types.json:1`.

## Backlog

- `crates/nvs-lsp/src/index.rs:@occurrences` records no use for a name written in type position, so
  `references` on `Owner::Name` lists nothing the type positions wrote.
- `crates/nvs-lsp/src/index.rs:@member_names` gives a `type` member no declaration row, so the
  outline nests one the reference index cannot name.
- `[context] modules` did not print `crates/nvs-lsp/src/document.rs` (`Analysed`, which every request
  reads) or `crates/nvs-lsp/src/render.rs` (how a symbol kind and a token are spelled in a test's
  expected string); both were needed to write stage 4's LSP work.
- `crates/nvs-fmt/src/lib.rs` known gap 5 now has a second reader: a shape type in type position is
  unspaced at both declaration sites — `docs/rules/tooling/fmt-novis-constructs.md` claims the space.
