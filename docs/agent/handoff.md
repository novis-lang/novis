# Handoff

## State

**Goal 14 answers five of the ten requests, and `definition` is the first that needed a name and a
type.** `crates/nvs-lsp/src/document.rs:291` keeps the `ExprTypeTable` the type phase fills and
`crates/nvs-lsp/src/document.rs:298` the `TypeInterner` it is read against, so a cursor answer is
the index saying which node an offset is in and that table saying what the node's name resolved to.
`crates/nvs-lsp/src/definition.rs` is the whole request: `New`, `NewDynamic`, `InstanceOf` and
`EnumCase` each name a class, and `nvs_hir::SymbolTable` has where it was declared. A member —
method or property — is `nvs_hir::MemberTable`'s and is not answered yet.

**`nvs lsp-test tests/lsp/` reports `33 passed, 0 failed`**, against the goal's floor of 160.

**The `.lspt` runner named the entry document by its absolute path**, because only a required file's
path is canonical and `Materialised::spelling` stripped one prefix. It strips either now
(`crates/nvs-lsp/src/suite.rs:244`); no expectation was edited.

**`semanticTokens/full` and `hover` are what stage 6 has left**, and both now have the table they
were waiting on.

## Next group

**Stage 6: the cursor requests that read a declaration** — one file set:
`crates/nvs-lsp/src/document.rs`, `crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/suite.rs`,
`crates/nvs-lsp/src/render.rs` and `crates/nvs-lsp/src/definition.rs`, plus one new sibling of it,
with `crates/nvs-syntax/src/ast.rs` and `crates/nvs-hir/src/members.rs` read only.

- [ ] **`hover`, starting with a declaration's doc comment.** The parser already attaches one:
      `crates/nvs-syntax/src/parser/decl.rs:451` calls `take_doc_comment` and the node holds it at
      `crates/nvs-syntax/src/ast.rs:1463`, so nothing here rescans trivia. What is missing is the
      step from the cursor to the declaration — `crates/nvs-lsp/src/definition.rs:68` already makes
      it as far as a `Symbol` and its `decl_span`. Render as Markdown through
      `crates/nvs-lsp/src/render.rs:270`. `rule:tooling/doc-comment-is-three-slashes`,
      `rule:ide/the-rendering-has-one-home`.
- [ ] **`hover` over a `Core` member, out of the registry's signature row.** ADR 0099 § 3's second
      hover source; the row is `crates/nvs-stdlib/src/registry.rs` and `rule:core-api/reference-card`
      is what guarantees every implemented member carries one. The dispatch to add it to is
      `crates/nvs-lsp/src/server.rs:160`.
- [ ] **`definition` for a method or a property.** `crates/nvs-lsp/src/definition.rs:82` reads four
      `ExprInfo` variants that name a class; `ExprInfo::Call` and `ExprInfo::Property` name a member,
      whose declaration is `nvs_hir::MemberTable`'s rather than `SymbolTable`'s — that table is keyed
      by a class label, which is the one thing to check before writing the arm.

## Backlog

- `semanticTokens/full`, the fifth projection — `docs/decisions/0099.md` § 4.
- `nvs/redactions`, the goal's stage 8 and the earliest acceptance check still red —
  `rule:security/redaction-ranges-come-from-the-server`.
- The two code actions, stage 9 — `docs/decisions/0099.md` § 3's closing paragraph.
- `completion`, which is the last request with real work behind it — ADR 0099 § 3.
- The `.lspt` corpus stands at 33 against a floor of 160; every request slice ships cases.
- `nvs lsp-test --coverage` and `every_request_answers_every_construct` are not written —
  `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.
