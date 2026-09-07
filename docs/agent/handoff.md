# Handoff

## State

**Goal 14: `hover` is whole, and stage 6 has `completion` left.** The three arms
`rule:ide/the-request-set-is-closed` names all live in `crates/nvs-lsp/src/hover.rs` and are tried
**per node, innermost first**: a `Core` member's registry row, the `///` run above the declaration the
name reached, then the declared type of the node itself. `crates/nvs-lsp/src/definition.rs:273`'s
`target_of` is the per-node name lookup both cursor requests share — a walk that found the nearest
*named* node first answered a call's card for a cursor sitting on one of its arguments.

**A `Core` member's signature line is spelled from the checker's types, never from `CoreTy`.**
`nvs_types::core_lib` seeded the signature table out of the registry row, so the `ResolvedCall` under
the cursor *is* that row interned and `nvs_types::TypeInterner::describe` is the one home for spelling
it; `nvs-cli`'s `ty_string` stays `nvs meta --json`'s. The reference card is read straight off
`nvs_stdlib::registry` (`rule:core-api/reference-card`), which is why `nvs-lsp` now names that crate.

**A plain variable read carries no type anywhere the server can reach.** The checker keeps a local's
type in a scope it drops and records nothing on the read, so `$total` hovers to nothing — and that
same gap is what stands in front of `completion` off `$u->`.

**`nvs lsp-test tests/lsp/` reports `44 passed, 0 failed`**, against the goal's floor of 160;
`verify.py` is 7 of 7 green.

**The goal's `[context.stage.6] adrs` names ADR 0088 § 5 for the signature row's rendering and that
section decides no rendering** — 0099 § 3 is the only record, and the spelling is now decided in
`crates/nvs-lsp/src/hover.rs`'s module doc.

## Next group

**Stage 6: `completion`, the last of the three** — one file set: a new
`crates/nvs-lsp/src/completion.rs`, `crates/nvs-lsp/src/suite.rs`, `crates/nvs-lsp/src/server.rs`,
`crates/nvs-lsp/src/case.rs`, with `crates/nvs-lsp/src/definition.rs`,
`crates/nvs-lsp/src/render.rs` and `crates/nvs-stdlib/src/registry.rs` read only. The rendering,
the `Request` variant and the capability all exist already — `crates/nvs-lsp/src/render.rs:302`
freezes `label kind detail` (`rule:ide/the-rendering-has-one-home`) and
`crates/nvs-lsp/src/capabilities.rs:164` already declares the provider.

- [ ] **Members off a resolved receiver, user classes and `Core` alike** —
      `rule:ide/the-request-set-is-closed`. Decide first where the receiver's class comes from:
      `$u-><|>` records no `ExprInfo` and `$u` itself carries none either (`## State` above), so the
      candidates are recording a type on a variable read in `nvs_types` — which costs a map entry per
      read on **every** compile, not only in the server — or keeping the checker's local scopes on
      `crates/nvs-lsp/src/document.rs:254`'s `Analysed`. Say which and why in the module doc; the
      lookup itself is `crates/nvs-lsp/src/definition.rs:273` and `crates/nvs-stdlib/src/registry.rs:1234`.
- [ ] **Enum cases after `Type::`, and a class's static members** — same module, same walk;
      `crates/nvs-lsp/src/suite.rs:187` and `crates/nvs-lsp/src/server.rs:183` are the two dispatch
      arms every one of these slices lands through.
- [ ] **Keywords filtered by position, and the variables in scope** — no workspace symbol search,
      which `rule:ide/five-features-are-one-reference-index` puts at M10. The roster is the lexer's
      own, `crates/nvs-syntax/src/lexer.rs:643`, and never a second list in this crate; the position
      that filters it is the ancestor path at `crates/nvs-lsp/src/definition.rs:256`.

## Backlog

- `semanticTokens/full` with ADR 0099 § 4's legend — stage 7's one remaining projection.
- `nvs/redactions`, the acceptance check that is red — `docs/agent/loop-goal.md` § *Stage 8*.
- The two code actions off `Diagnostic::suggestions` — `docs/agent/loop-goal.md` § *Stage 9*.
- A `@see` target renders as prose, not a link — `crates/nvs-lsp/src/hover.rs`'s module doc.
- Hover over a *user* method call answers its run or nothing, never its signature — same module doc.
- The full-reanalysis latency bound — `rule:ide/a-full-reanalysis-stays-under-a-bound`, stage 10.
