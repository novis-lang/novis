# Handoff

## State

**Goal 14: stage 6's `completion` answers its first arm — members off a resolved receiver.**
`crates/nvs-lsp/src/completion.rs` is the module: the cursor's `NodePath` is searched for a
`PropertyAccess`/`MethodCall`, the receiver is that node's **first child** through the new
`nvs_syntax::SyntaxIndex::children_of`, and a cursor at or past the receiver's end is what "in the
member half" means. Nothing reads the written member name — `$u->` followed by `if (true) {` parses
as one method call whose member is `if`, which is the grammar being resilient rather than a bug.

**The checker's local scopes now outlive the frame that built them**, which is the decision the item
asked for and the module doc argues: `nvs_types::ExprTypeTable::local_scopes` takes one entry per
*declaration*, moved out of a `LocalScope` that was about to be dropped, instead of one entry per
variable *read* on every compile. `crates/nvs-types/src/check.rs`'s `record_locals` is the one
recording helper and there are four call sites — the file's synthesized frame, a hook body, a method
body and a closure body. That is also what stage 6's third arm (the variables in scope) will read.

**A `CoreTy` spells itself.** `nvs_stdlib::registry::CoreTy::spelled` is the one home; `nvs meta
--json`'s `ty_string` is now a one-line call into it, and `completion` uses it for a `Core` instance
row's detail column. A user member's detail is the **source text of its declaration** instead —
`crate::completion`'s module doc holds why, and why the two are not one spelling.

**`nvs lsp-test tests/lsp/` reports `49 passed, 0 failed`**, against the goal's floor of 160;
`verify.py` is 7 of 7 green. Inherited members and visibility are the two known gaps, both named in
`crates/nvs-lsp/src/completion.rs`'s module doc.

## Next group

**Stage 6: the rest of `completion`** — one file set: `crates/nvs-lsp/src/completion.rs`, with
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-types/src/expr_table.rs` and
`crates/nvs-lsp/src/render.rs` read only, plus new cases under `tests/lsp/completion/`.

- [ ] **Enum cases after `Type::`, and a class's static members** —
      `rule:ide/the-request-set-is-closed`. `crates/nvs-lsp/src/completion.rs:92`'s `MEMBER_ACCESS`
      is the roster to widen with `StaticCall`/`ClassConst`, and the receiver of one of those is a
      *class name* rather than a value, so `crates/nvs-lsp/src/completion.rs:114` needs a second arm
      that resolves the written name instead of asking for a type — `nvs_types::ExprInfo::EnumCase`
      and `crates/nvs-lsp/src/definition.rs:169` are what it reads.
      `crates/nvs-stdlib/src/registry.rs:1276`'s `methods` is the `Core` static roster.
- [ ] **Keywords filtered by position, and the variables in scope** —
      `rule:ide/the-request-set-is-closed`, no workspace symbol search.
      `crates/nvs-types/src/expr_table.rs:1108`'s `local_scopes` already answers the second half:
      innermost body containing the cursor, every binding in it, labelled with the `$` sigil an
      editor inserts. `crates/nvs-lsp/src/completion.rs:101`'s `at` answers nothing for a cursor in
      no access, and that is where the bare-position arm goes.

## Backlog
- Inherited members are not offered — `crates/nvs-lsp/src/completion.rs`'s module doc owns the gap.
- Visibility is not applied, so a `private` member is offered outside its class — same doc.
- The coverage matrix (`nvs lsp-test --coverage`) is unwritten — `docs/agent/loop-goal.toml:5230`.
- `nvs/redactions` (stage 8) is the earliest failing acceptance check — `docs/agent/loop-goal.toml:5259`.
- `semanticTokens` has no `suite.rs` arm yet — `crates/nvs-lsp/src/suite.rs:185`.
