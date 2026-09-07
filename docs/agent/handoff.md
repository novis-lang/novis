# Handoff

## State

**Goal 14, stage 7's producer landed: `textDocument/semanticTokens/full` answers**, in the `.lspt`
suite and in the server both. `crates/nvs-lsp/src/semantic.rs` is the walk;
`crates/nvs-lsp/src/capabilities.rs:110`'s legend is unchanged and read only.

**The walk emits a token only where the tree itself answers what a name is** — a declaration, a
position only one kind may stand in (`extends`, `implements`, `new C()`, the receiver of `C::m()`),
and a member access. Nothing here resolves a written name, which is `crates/nvs-lsp/src/definition.rs`'s
standing rule. The silences that follow from it are each argued in the module doc: a type
annotation, a class constant or enum-case access, `instanceof`'s right-hand side, an attribute, a
`use` target, and `typeParameter`, which no production declares.

**Every token's modifier bitset is 0** — `defaultLibrary`, `tainted` and `secret` are the next item,
and no `.lspt` case freezes a modifier yet, so adding them appends to expectations rather than
rewriting them.

`nvs lsp-test tests/lsp/` reports `65 passed, 0 failed`, against the goal's floor of 160;
`verify.py` is green. `semanticTokens types=` narrows the answer **before** it is encoded
(`crates/nvs-lsp/src/semantic.rs:205`), because the wire's deltas are relative and a filter applied
after would move every token following a dropped one.

## Next group

**Stage 7: the two modifiers this layer exists for, and the silence a rejection owes** — one file
set: `crates/nvs-lsp/src/semantic.rs`, with `crates/nvs-lsp/src/capabilities.rs` and
`crates/nvs-lsp/src/document.rs` read, and new cases under `tests/lsp/semantic/`.

- [ ] **`defaultLibrary` on a `Core` class** — `rule:ide/semantic-tokens-carry-the-qualifiers`,
      ADR 0099 §4. `crates/nvs-lsp/src/capabilities.rs:66`'s `TOKEN_MODIFIERS` is the wire order and
      is read only; `crates/nvs-lsp/src/semantic.rs:667`'s `receiver` is where `Core\Str::upper()`
      already earns a `class` token, and `crates/nvs-lsp/src/definition.rs:288`'s `target_of` is how
      this crate reads the checker's own resolution off `crates/nvs-lsp/src/document.rs:291` rather
      than resolving a name a second time.
- [ ] **`tainted` and `secret` at every use site** — `rule:security/tainted-qualifier`,
      `rule:security/secret-qualifier`. The qualifier is a *type*, so the answer is
      `crates/nvs-lsp/src/document.rs:291`'s `exprs` keyed by expression span and
      `crates/nvs-lsp/src/document.rs:298`'s interner beside it; `crates/nvs-lsp/src/semantic.rs:246`'s
      `Named` collects `(Span, Kind)` pairs today and is what grows a modifier field.
- [ ] **A rejected construct gets no colour** — `rule:ide/rejected-syntax-gets-no-colour`.
      `crates/nvs-lsp/src/semantic.rs:422` already refuses to dress a top-level `function` as a
      method; the remaining question is `$u->if(...)`, which the parser accepts as a member name and
      this walk therefore colours `method` — decide there whether the walk narrows or the parser does.

## Backlog

- A type annotation's name is uncoloured; closing it needs `nvs_hir::hierarchy::resolve_ref` at
  `crates/nvs-hir/src/hierarchy.rs:335` plus the site's namespace and imports — `crates/nvs-lsp/src/semantic.rs`'s module doc.
- `Config::MAX` and `Status::Draft` are one production and both silent; `ExprInfo::EnumCase` is what
  tells them apart — same module doc.
- `tests/lsp/semantic/` is the directory name, not the `semantic-tokens/` the previous handoff
  guessed, so it matches its sibling `symbols/` and `folding/` — `tests/lsp/README.md`.
- Stage 8's `nvs/redactions` has no producer yet; its three named tests are the acceptance check
  that has been failing since the goal opened — `docs/agent/loop-goal.toml:5274`.
