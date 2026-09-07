# Handoff

## State

**Goal 14, stage 7 is whole bar its last item: a semantic token now carries all three
modifiers.** `crates/nvs-lsp/src/semantic.rs`'s `Modifier` is the legend's three, each finding its own
bit in `crates/nvs-lsp/src/capabilities.rs:66` the way `Kind` finds its type index, so the wire order
is written once.

**Every modifier is a resolution, never a reading of the written name.** `defaultLibrary` comes from
`crate::definition::target_of` over the *enclosing* expression's entry — a receiver is never an entry
of its own — and `tainted`/`secret` from `nvs_types::expr::quals::is_tainted`/`is_secret`, which are
now `pub` so the editor and the checker cannot disagree about what a value carries.

**The qualifier question is asked in one place**, `crates/nvs-lsp/src/semantic.rs:348`'s `push`, for
every `Kind::Variable` token however the walk reached it — that is what makes "at every use site" a
property of the code rather than of eight call sites remembering. A parameter is the one binding no
body's scope covers, so `param` reads its written annotation through
`nvs_types::ExprTypeTable::declared_ty` instead.

`Analysed::local_ty` and `Analysed::bodies_at` moved to `crates/nvs-lsp/src/document.rs:353`, since
completion and semantic tokens both ask them now. `nvs lsp-test tests/lsp/` reports `68 passed,
0 failed` against the goal's floor of 160; `verify.py` is green.

## Next group

**Stage 7's tail, then the qualifier reach it exposed** — one file set:
`crates/nvs-lsp/src/semantic.rs`, with `crates/nvs-lsp/src/document.rs` read, and new cases under
`tests/lsp/semantic/`.

- [ ] **A rejected construct gets no colour** — `rule:ide/rejected-syntax-gets-no-colour`, ADR 0099
      §4. `crates/nvs-lsp/src/semantic.rs:714`'s `expr` ends in a wildcard that already emits nothing
      for a production it does not name, so this slice is a `.lspt` case per refused construct
      proving it — `===`, `(int)$x`, `|>`, `if (...): … endif;` — beside
      `tests/lsp/semantic/a-refused-free-function-is-not-dressed-as-a-method.lspt`, which is the one
      that exists. Check the wildcard actually reaches each: a construct the *parser* recovers into
      some other node would be coloured as that node.
- [ ] **A `foreach` binding over a qualified element carries nothing** —
      `rule:security/tainted-qualifier`. `foreach ([$body] as $chunk)` colours `$chunk` bare while
      `$body` is `tainted`, which is the direction the rule cannot afford:
      `crates/nvs-lsp/src/semantic.rs:412`'s `qualifiers_at` asks
      `crates/nvs-lsp/src/document.rs:353`'s `local_ty`, so the question is whether a `foreach`
      binding is in `nvs_types`' `LocalScope` at all, or is there with the array's own type.
- [ ] **A property access carries its qualifier too** — same rule.
      `crates/nvs-lsp/src/semantic.rs:412` handles a variable and nothing else, so `$this->token`
      declared `secret string` colours bare. The type is on the access's own entry
      (`nvs_types::ExprInfo::Property`), which `crates/nvs-lsp/src/hover.rs:130` already reads.

## Backlog

- `qualifiers_of` reads one atom, so `?tainted string` (a union) sets no bit — widening it is a
  question for `nvs_types::expr::quals`, which has `carries_tainted` and no `carries_secret`.
- `typeParameter` is in the legend and emitted nowhere; no production declares one — `semantic.rs`'s
  module doc owns why.
- The client half of the legend and the `semanticTokenScopes` mapping are goal 15's —
  `rule:ide/novis-ships-names-not-colours`.
- `nvs lsp-test` is at 68 of the goal's floor of 160 cases; `python tools/gaps.py` ranks what is thin.
