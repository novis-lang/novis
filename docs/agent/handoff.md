# Handoff

## State

**Goal 14, stage 6 is whole: `hover`, `definition` and `completion` all answer.** `completion`'s
last arm landed — a cursor in no access is a *position*, and
`crates/nvs-lsp/src/completion.rs:329`'s `position` answers one of three, told apart by the node the
index says the cursor is **directly** inside: a body of statements gets the words that may open one
plus the variables in scope, a class or interface body gets the ten words that may open a member, an
enum body gets `case`, and a member's own node (a property, a class constant, an enum case) gets
nothing.

**The three word lists are copies of the grammar, guarded rather than derived.**
`crates/nvs-lsp/src/completion.rs:254` reads them out of `nvs_syntax::parser`'s statement and
primary-expression dispatches, minus every arm either routes to an `E02xx` rejection; a `#[test]` at
the foot of the file fails on a spelling `nvs_syntax::Keyword::from_lowercase` does not know.
Completeness is judgement and is not claimed.

**`nvs lsp-test tests/lsp/` reports `60 passed, 0 failed`**, against the goal's floor of 160;
`verify.py` is 7 of 7 green. The known gaps are the module doc's five, and the two new ones are the
receiver half of an access (a variable is being written there and none is offered) and a method's
own parameter list (answered as the body it precedes).

## Next group

**Stage 7: `semanticTokens/full`, the one projection with no producer** — one file set: a new
`crates/nvs-lsp/src/semantic.rs`, with `crates/nvs-lsp/src/capabilities.rs`,
`crates/nvs-lsp/src/render.rs` and `crates/nvs-lsp/src/suite.rs` edited or read, plus new cases under
`tests/lsp/semantic-tokens/`. The legend and the capability are already declared — this is the walk
that fills them.

- [ ] **The walk that emits one token per name, against the legend already declared** —
      `rule:ide/semantic-tokens-carry-the-qualifiers`, ADR 0099 §4.
      `crates/nvs-lsp/src/capabilities.rs:110`'s `semantic_tokens_legend` is the token types and
      modifiers in wire order and is read only; `crates/nvs-lsp/src/render.rs:318` already renders
      `L:C+len type modifiers` with the deltas undone, so nothing new is needed to freeze one.
- [ ] **The `.lspt` arm, so a case can ask for it** — `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.
      `crates/nvs-lsp/src/suite.rs:185`'s `answer` is the dispatch every request slice lands one arm
      in, and `Request::SemanticTokens` is the one it still refuses;
      `crates/nvs-lsp/src/case.rs:65` is the name a `--REQUEST--` line writes.
- [ ] **`defaultLibrary` on a `Core` class, and the `tainted` and `secret` modifiers** —
      `rule:ide/semantic-tokens-carry-the-qualifiers`, which the goal's stage 7 calls the point of
      the item rather than a detail of it. Same file, `crates/nvs-lsp/src/completion.rs:204`'s
      `named_class` is the worked example of asking the checker first and the written name second.
- [ ] **A rejected construct gets no colour** — `rule:ide/rejected-syntax-gets-no-colour`. The walk
      in the first item is where the refusal lives, and what it is refusing is the node
      `crates/nvs-syntax/src/walk.rs:580` spells: a rejected shape is parsed, reported and left as an
      `Error` node, so the tokens under one are what must not be emitted. One case per shape under
      `tests/lsp/semantic-tokens/`.

## Backlog

- `nvs_syntax::Keyword`'s doc comment promises a `name()` that does not exist —
  `crates/nvs-syntax/src/token.rs:317`, and the playbook bullet is what stands in for it.
- An inherited member and a `private` one: both are the class `completion` reached, not its walk —
  `crates/nvs-lsp/src/completion.rs`'s module doc *Known gaps*.
- `self::`, `static::` and `parent::` offer nothing when the member half is still empty — same.
- The receiver half of an access offers no variable, though one is being written there — same.
- Stage 8's `nvs/redactions` is the driver's earliest red check, and its artefact is not written yet.
- `crates/nvs-lsp/src/suite.rs`'s `--coverage` matrix is the M4B gate nothing has run against the
  position arm yet.
