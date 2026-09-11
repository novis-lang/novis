# Handoff

## State

**Goal `workspace-index`, stage 5 is closed.** The PHP-name arm is
`crates/nvs-lsp/src/completion.rs`'s `php_builtins`, offered beside the bare names at a statement
position: `nvs_stdlib::php_names::starting_with` answers for the run of name characters before the
cursor, and `php_item` turns each `Candidate::items()` entry into an item whose `insert_text` is
`Item::insertion()` — or, for the three shapes that insert nothing, exactly the characters already
typed, because a client replaces the word being completed with that field and the PHP spelling may
never reach a file.

**The arm answers a name, not a cursor.** `PHP_PREFIX` characters are written first (three), which is
this session's call and is documented where the constant is: a one- or two-character prefix reaches
hundreds of built-ins and buries the program's own names. One `.lspt` case had to narrow its
`prefix=` because of it, and one new case freezes the layer's rendering.

**`nvs.completion.phpNames` is landed on both sides** — `crates/nvs-lsp/src/settings.rs`'s
`PhpNames` (`all`/`resolved`/`off`, default `all`), threaded to the arm through `completion::at`, and
contributed in `editors/vscode/package.json` with its row in `docs/reference/tools/40-editor.md`.
`rule:ide/the-extension-claims-nvs-only`'s two guards used to scan the manifest for the three letters
and now read the claims — a language id, a file extension, an activation event — because a frozen
setting name says `php` and claims nothing.

**Stage 6 is all that is left, and it is the driver's failing check.**

## Next group

**Stage 6: inlay hints, the two settled shapes** — one file set: `crates/nvs-lsp/src/hints.rs` (new),
`crates/nvs-lsp/src/capabilities.rs`, `crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/case.rs`,
`crates/nvs-lsp/src/render.rs`, `crates/nvs-lsp/tests/hints.rs` (new), `tests/lsp/hints/`.

- [ ] **The two shapes.** A new `crates/nvs-lsp/src/hints.rs` reading what the type phase already
      recorded — `crates/nvs-lsp/src/document.rs:460`'s `exprs` — for the inferred type after a `var`
      declaration with no annotation, and the parameter name at a call site whose argument is a bare
      literal. Nothing else: `loop-goal.md` § *Stage 6* bounds the reversal to the two idioms that
      have stopped moving, and [ADR 0099](../decisions/0099.md) § 3 is the deferral being reversed.
      The acceptance names `an_inferred_var_declaration_carries_its_type_as_a_hint`,
      `a_literal_argument_carries_its_parameter_name` and
      `no_hint_is_produced_from_anything_the_type_phase_did_not_record`, which is the design.
- [ ] **The wire.** `crates/nvs-lsp/src/capabilities.rs:260`'s `declared_capabilities` gains
      `inlayHintProvider`, and `crates/nvs-lsp/src/server.rs:283`'s dispatch gains the arm beside
      `completion`. `rule:ide/the-request-set-is-closed` names inlay hints as M10's and is the rule
      the goal's ADR amends.
- [ ] **The freezing.** `crates/nvs-lsp/src/case.rs:63`'s `Request` gains the request,
      `crates/nvs-lsp/src/render.rs:135`'s `Response` gains its canonical rendering, and the cases go
      under `tests/lsp/hints/`. `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`. Run
      `target/debug/nvs.exe lsp-test tests/lsp/ --coverage` — `cargo test` does not.

## Backlog

- The goal's one ADR is unopened and owes six stages' reasoning — `loop-goal.md` § *Standing decisions*.
- `nvs.check.scope` and `nvs.codeLens.enable` are read by the server and contributed by neither the
  manifest nor the chapter — `rule:ide/contributions-are-frozen-and-only-ever-added`.
- A `NotRegistered` item cannot name the milestone it waits on: only `tests/migration-members-outstanding.txt` knows, and it is a test fixture — `php_names.rs` module doc.
- A destination naming an interface (`Core\Db\Queryable`) inserts nothing, since the interface has no registry row — same.
- A cursor in a type annotation or a parameter list reaches no arm of its own — `completion.rs` § *Known gaps*.
- `registry`'s six global interfaces are reachable by no arm: they have no namespace to be under — same.
