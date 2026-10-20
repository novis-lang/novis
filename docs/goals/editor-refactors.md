---
looper: 1
title: The editor renames, finds, imports and generates
summary: "Workspace rename, workspace symbol search, organize imports, the two extractions and the three generators answer as their rules say, nvs check warns about an unused use and removes it with --fix, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-lsp/src/capabilities.rs, crates/nvs-lsp/src/index.rs, docs/rules/ide/the-request-set-is-closed.md, docs/rules/ide/an-lsp-answer-is-frozen-as-an-lspt-case.md]
stages:
  - id: 1
    title: "rename across the workspace, and a file rename updates `require` paths"
    status: open
    context: [crates/nvs-lsp/src/capabilities.rs, crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/rename.rs, crates/nvs-lsp/src/document.rs, crates/nvs-lsp/src/case.rs, docs/rules/ide/no-refactoring-introduces-an-alias.md, docs/rules/ide/five-features-are-one-reference-index.md]
    extra:
      context:
        rules: [ide/no-refactoring-introduces-an-alias, ide/five-features-are-one-reference-index, statements/require-is-the-only-inclusion-construct, ide/the-request-set-is-closed]
  - id: 2
    title: "workspace symbol search, organize imports, and a warning for an unused `use`"
    status: open
    context: [crates/nvs-lsp/src/capabilities.rs, crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/imports.rs, crates/nvs-fmt/src/imports.rs, crates/nvs-lsp/src/diagnostics.rs, crates/nvs-diagnostics/src/code/warnings.rs, crates/nvs-cli/src/check.rs, docs/rules/ide/no-refactoring-introduces-an-alias.md, docs/rules/tooling/fmt-sorts-the-use-block.md, docs/rules/tooling/fmt-is-never-a-diagnostic.md]
    extra:
      context:
        rules: [ide/no-refactoring-introduces-an-alias, statements/nothing-gets-a-second-name, tooling/fmt-sorts-the-use-block, ide/the-request-set-is-closed, statements/an-unused-use-is-a-warning, tooling/fmt-is-never-a-diagnostic, core-api/casing-has-no-suppression]
  - id: 3
    title: "extract to variable and extract to method"
    status: open
    context: [crates/nvs-lsp/src/actions.rs, crates/nvs-lsp/src/extract.rs, crates/nvs-lsp/src/server.rs, docs/rules/ide/no-refactoring-introduces-an-alias.md]
    extra:
      context:
        rules: [ide/no-refactoring-introduces-an-alias, ide/a-code-action-writes-only-what-is-already-determined]
  - id: 4
    title: "implement missing members, and override a method"
    status: open
    context: [crates/nvs-lsp/src/actions.rs, crates/nvs-lsp/src/generate.rs, crates/nvs-lsp/src/completion.rs, docs/rules/ide/a-code-action-writes-only-what-is-already-determined.md]
    extra:
      context:
        rules: [ide/a-code-action-writes-only-what-is-already-determined, statements/inout-is-the-by-reference-spelling]
  - id: 5
    title: "declare the function you called"
    status: open
    context: [crates/nvs-lsp/src/generate.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-diagnostics/src/diagnostic.rs, crates/nvs-diagnostics/src/code/types.rs, docs/rules/ide/a-code-action-writes-only-what-is-already-determined.md]
    extra:
      context:
        rules: [ide/a-code-action-writes-only-what-is-already-determined]
checks:
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, editor-refactors]
    reach: true
    floor: false
---

# The editor renames, finds, imports and generates

## Why

The language server answers references, highlights, lenses, signature help and inlay hints from
one reference index, but it offers none of the refactorings
`rule:ide/no-refactoring-introduces-an-alias` names. `server_capabilities` leaves `rename` and
`workspaceSymbol` unset (`crates/nvs-lsp/src/capabilities.rs:162`). There is no organize imports, no
extraction, and none of the three generators of
`rule:ide/a-code-action-writes-only-what-is-already-determined`. This goal builds them, in the order
each one's dependency lands.

## Standing decisions

**Member resolution is completeness-holes' stage 10, not this goal's.** Goal `completeness-holes`
built it: completion walks a class's parents and interfaces and filters each member by visibility.
Stage 4's generators read the members that walk resolves (`crates/nvs-lsp/src/completion.rs:3078`,
`declared_members`). Gate: that function exists, so the gate holds.

**The narrowing action is goal `editor-fixes`' work.**
`rule:ide/a-code-action-writes-only-what-is-already-determined` names four writing actions. This
goal builds three, so the rule ships with the fourth and names goal `editor-fixes` as its owner.
This goal flips only `rule:ide/no-refactoring-introduces-an-alias`.

**No refactoring writes an alias.** `rule:ide/no-refactoring-introduces-an-alias` binds every stage.
A rename that would make two names clash refuses with a message. It never writes `use ... as`.

**Extract to method is offered only for a selection with one exit and no write that escapes.** A
selection that assigns a local read after it, or that holds a `return`, `break` or `continue`
leaving it, is offered no extract to method. Extract to variable is unaffected. Why: a code action
writes only what is determined, and guessing `inout` or a result tuple is the simplicity cost the
rule exists to refuse.

**Each refactoring is its own module.** Rename is `crates/nvs-lsp/src/rename.rs`, the extractions
`extract.rs`, the three generators `generate.rs`. `actions.rs:150` gains one call per module. Why:
stages 3 to 5 then share one line of `actions.rs`, not its body.

**An unused `use` is a warning that `nvs check` reports.** It gets a new code, the next free
`W10xx` in `crates/nvs-diagnostics/src/code/warnings.rs` (`W1025` when this was written, but
`compiler-holes` adds codes, so the session reads the file first). `nvs check` reports it, so the edit
hook an agent runs shows it. Stage 2 picks its source and records why in the rule: the unused-import
index the editor dims with (`crates/nvs-lsp/src/index.rs:498`, `crates/nvs-lsp/src/diagnostics.rs:316`),
or the name resolution of `nvs-hir`. Safe default: the index, because the editor and `nvs check` then
cannot disagree.

- The editor shows one diagnostic per unused `use`: the warning, with the `Unnecessary` tag that
  dims it. The separate dimming hint for an import goes. Why: one mistake, one entry in the problems
  list.
- The warning carries a safe fix that deletes the line, so `nvs check --fix` removes it
  (`crates/nvs-cli/src/check.rs:94`).
- `nvs fmt` keeps the line. Why: `nvs fmt` is layout only and has no name resolution
  (`rule:tooling/fmt-is-never-a-diagnostic`).
- The rule is `statements/an-unused-use-is-a-warning`, a new fragment. Three rule sentences say that
  Novis has no warning tier, and the `W10xx` tier exists (`W1003`, `W1023`). Stage 2 rewrites them to
  say what is true: a construct the language does not allow is always an error, and a warning is for
  code that runs as written.

**Lanes.** Stages 1 and 2 share `capabilities.rs` and `index.rs`, so 2 runs after 1. Stage 3
(`extract.rs`) is disjoint from 1 and 2 apart from one capability line, and can run in any order
with them. Stages 4 and 5 share `generate.rs`, so 5 runs after 4. Stage 5 also touches
`nvs-types` and `nvs-diagnostics`. Stage 2's warning touches `warnings.rs` and `nvs check`'s
`--fix`, which no other stage here touches. The goal changes no program's behaviour and can run
beside the chain as a side goal.

**Trade-offs.** No program changes, and nothing runs on the request path. Rename and symbol search
read the reference index the server already holds, so they add no store. The index answers
per-request work in the editor, and its memory does not change. Each action is one more answer in
the closed request set, which `rule:ide/the-request-set-is-closed` lists, so the protocol surface
grows by the requests named here and no others. Stage 5 adds the argument types to one diagnostic,
a few words per unresolved call, only while the checker reports it. The unused-`use` warning adds
one diagnostic per unused line to `nvs check`, and nothing to a program that compiles and runs.

## Stages

### 1. rename across the workspace, and a file rename updates `require` paths

**Does:** `textDocument/rename` with `prepareRename` renames a symbol at every occurrence the index
holds, and `workspace/willRenameFiles` rewrites each `require` path that names a renamed file.

**Split:** after the handlers land, one writing subagent writes the `.lspt` cases under
`tests/lsp/rename/`: a local, a method across files, a `Core` name refused, a clash refused, a
renamed file's `require`.

- [ ] `prepareRename` refuses a position that is no renamable symbol, a `Core` name among them. `crates/nvs-lsp/src/rename.rs:1`, `crates/nvs-lsp/src/index.rs:833`
- [ ] `rename` edits every occurrence from the index, and refuses a new name that clashes in any file it touches (`rule:ide/no-refactoring-introduces-an-alias`). `crates/nvs-lsp/src/rename.rs:1`, `crates/nvs-lsp/src/index.rs:629`
- [ ] `willRenameFiles` returns the `require` path edits, from the require graph and each `require`'s recorded target (`rule:statements/require-is-the-only-inclusion-construct`). `crates/nvs-lsp/src/rename.rs:1`, `crates/nvs-lsp/src/document.rs:366`, `crates/nvs-types/src/expr_table.rs:2324`
- [ ] The three providers declared, and their requests routed. `crates/nvs-lsp/src/capabilities.rs:162`, `crates/nvs-lsp/src/server.rs:569`
- [ ] The handshake test holds the new capability list. `crates/nvs-lsp/tests/handshake.rs:109`
- [ ] The `.lspt` runner learns `rename` and `willRenameFiles`. `crates/nvs-lsp/src/case.rs:59`
- [ ] `.lspt` cases for each. `tests/lsp/rename/a-local-is-renamed-at-every-occurrence.lspt:1`
- [ ] `rule:ide/the-request-set-is-closed` lists the three new requests. `docs/rules/ide/the-request-set-is-closed.md:1`

### 2. workspace symbol search, organize imports, and a warning for an unused `use`

**Does:** `workspace/symbol` finds a declaration in any indexed file, organize imports reorders
the `use` block and removes unused lines, and `nvs check` warns about an unused `use` and removes it
with `--fix`.

**Split:** after both land, one writing subagent writes the `.lspt` cases under `tests/lsp/symbols/`
and `tests/lsp/actions/`.

The unused-`use` warning, which reads the index organize imports also reads:

- [ ] The rule fragment and its `designed` record, with the source the session picked and why (`statements/an-unused-use-is-a-warning`). `docs/rules/statements/an-unused-use-is-a-warning.md:1`, `data/rules/statements/an-unused-use-is-a-warning.json:1`
- [ ] The next free `W10xx` code, with its card and its help text, which follows AGENTS.md § *Text an end user reads*. `crates/nvs-diagnostics/src/code/warnings.rs:1`
- [ ] `nvs check` reports it for each unused `use`, from the index or from name resolution. `crates/nvs-cli/src/check.rs:1`, `crates/nvs-lsp/src/index.rs:498`
- [ ] It carries a safe fix that deletes the line, and `nvs check --fix` applies it. `crates/nvs-cli/src/check.rs:94`, `crates/nvs-diagnostics/src/diagnostic.rs:115`
- [ ] The editor shows the warning with the `Unnecessary` tag, and the separate import dimming goes. `crates/nvs-lsp/src/diagnostics.rs:316`
- [ ] Tests: an unused `use` warns, a used one does not, `--fix` removes the line and the program still compiles, and `nvs fmt` keeps it. `crates/nvs-cli/tests/check_flags.rs:1`
- [ ] The three sentences that say Novis has no warning tier say what is true. `docs/rules/core-api/casing-has-no-suppression.md:1`, `docs/rules/core-api/written-visibility.md:1`, `docs/rules/types/no-legacy-cast.md:1`
- [ ] The rule `shipped` with the tests in `guardedBy`. `data/rules/statements/an-unused-use-is-a-warning.json:1`

Workspace symbol search and organize imports:

- [ ] `workspace/symbol` answers from the index. `crates/nvs-lsp/src/capabilities.rs:162`, `crates/nvs-lsp/src/index.rs:655`, `crates/nvs-lsp/src/server.rs:569`
- [ ] Organize imports is a `source.organizeImports` action built on the edit machinery of `nvs/importEdits`. It only reorders and removes (`rule:ide/no-refactoring-introduces-an-alias`). `crates/nvs-lsp/src/imports.rs:188`, `crates/nvs-lsp/src/imports.rs:241`, `crates/nvs-lsp/src/index.rs:498`
- [ ] Its order is the order `rule:tooling/fmt-sorts-the-use-block` gives, so a format after it changes nothing. `crates/nvs-fmt/src/imports.rs:87`
- [ ] The `.lspt` runner learns `workspaceSymbol`, and the handshake test holds the provider. `crates/nvs-lsp/src/case.rs:59`, `crates/nvs-lsp/tests/handshake.rs:109`
- [ ] `.lspt` cases for each. `tests/lsp/symbols/a-workspace-search-finds-a-class-in-another-file.lspt:1`, `tests/lsp/actions/organize-imports-sorts-and-removes-unused-lines.lspt:1`
- [ ] `rule:ide/the-request-set-is-closed` lists `workspace/symbol` and the action. `docs/rules/ide/the-request-set-is-closed.md:1`

### 3. extract to variable and extract to method

**Does:** two `refactor.extract` actions over a selection.

**Split:** after both land, one writing subagent writes the `.lspt` cases under `tests/lsp/actions/`,
among them each refused shape § *Standing decisions* names.

- [ ] Extract to variable writes a typed local declaration from the type the checker recorded for the selection, and replaces the selection with the variable. `crates/nvs-lsp/src/extract.rs:1`, `crates/nvs-lsp/src/actions.rs:150`
- [ ] Extract to method writes a method whose parameters are the locals the selection reads, typed as the checker recorded them, and whose return type is the selection's. A selection with an escaping write or an early exit is offered nothing. `crates/nvs-lsp/src/extract.rs:1`, `crates/nvs-lsp/src/document.rs:619`
- [ ] Neither action is offered where a type is not fully known (`rule:ide/a-code-action-writes-only-what-is-already-determined`). `crates/nvs-lsp/src/extract.rs:1`
- [ ] The rule states the extract-to-method boundary. `docs/rules/ide/no-refactoring-introduces-an-alias.md:1`
- [ ] `.lspt` cases for each. `tests/lsp/actions/a-typed-expression-extracts-to-a-variable.lspt:1`
- [ ] `rule:ide/no-refactoring-introduces-an-alias` `shipped`, once stages 1 to 3 are done. `data/rules/ide/no-refactoring-introduces-an-alias.json:3`

### 4. implement missing members, and override a method

**Does:** two of the three generators of `rule:ide/a-code-action-writes-only-what-is-already-determined`,
each writing only what the type system already determined.

**Split:** after both land, one writing subagent writes the `.lspt` cases under `tests/lsp/actions/`.

- [ ] Implement missing members inserts every missing method and property with its declared signature, `inout` and qualifiers kept, and a body that throws. `crates/nvs-lsp/src/generate.rs:1`, `crates/nvs-lsp/src/actions.rs:150`, `crates/nvs-lsp/src/completion.rs:3078`
- [ ] Override a method offers the hierarchy's overridable members and inserts the chosen signatures with a call to the base. `crates/nvs-lsp/src/generate.rs:1`, `crates/nvs-lsp/src/index.rs:548`
- [ ] Each one applied twice is a no-op, and a class with the first one applied passes `nvs check`. `crates/nvs-lsp/src/generate.rs:1`
- [ ] `.lspt` cases for each. `tests/lsp/actions/a-class-missing-an-interface-method-is-offered-its-signature.lspt:1`

### 5. declare the function you called

**Does:** the third generator, over the unresolved-call diagnostic.

**Split:** one writing subagent writes the `.lspt` cases under `tests/lsp/actions/` after the action
lands.

- [ ] The rule says that it declares a method on the receiver's class, because Novis has no free function (`E0215`, `E0320`). `docs/rules/ide/a-code-action-writes-only-what-is-already-determined.md:1`, `crates/nvs-diagnostics/src/code/rejected_php.rs:87`, `crates/nvs-diagnostics/src/code/names.rs:148`
- [ ] The unresolved-method diagnostic carries the argument types and the return type the checker computed. `crates/nvs-types/src/expr/calls.rs:1819`, `crates/nvs-diagnostics/src/code/types.rs:40`, `crates/nvs-diagnostics/src/diagnostic.rs:1`
- [ ] The action writes that method, applied twice is a no-op. `crates/nvs-lsp/src/generate.rs:1`
- [ ] `.lspt` cases for it. `tests/lsp/actions/an-unknown-method-call-is-offered-its-declaration.lspt:1`
- [ ] `bun nv owners --closes editor-refactors` passes. `data/rules/ide/no-refactoring-introduces-an-alias.json:3`

## Handoff

### State

No stage is started. The anchors above were checked against the tree. The member walk stage 4
reads exists, and the extract-to-method boundary is settled in § *Standing decisions*.

### Next

**Stage 1: rename**, one file set: `crates/nvs-lsp/src/rename.rs`, `capabilities.rs`, `index.rs`,
`server.rs`, `document.rs`, `case.rs`, `tests/lsp/rename/`. No gate.

- [ ] `prepareRename` refuses a non-symbol and a `Core` name (`rule:ide/no-refactoring-introduces-an-alias`). `crates/nvs-lsp/src/rename.rs:1`, `crates/nvs-lsp/src/index.rs:833`
- [ ] `rename` from the index, refusing a clash. `crates/nvs-lsp/src/index.rs:629`
- [ ] `willRenameFiles` rewrites `require` paths (`rule:statements/require-is-the-only-inclusion-construct`). `crates/nvs-lsp/src/document.rs:366`, `crates/nvs-types/src/expr_table.rs:2324`
- [ ] Providers declared and routed. `crates/nvs-lsp/src/capabilities.rs:162`, `crates/nvs-lsp/src/server.rs:569`
- [ ] Handshake test. `crates/nvs-lsp/tests/handshake.rs:109`
- [ ] The `.lspt` runner learns both requests. `crates/nvs-lsp/src/case.rs:59`
- [ ] `.lspt` cases. `tests/lsp/rename/a-local-is-renamed-at-every-occurrence.lspt:1`
- [ ] `rule:ide/the-request-set-is-closed` lists the new requests. `docs/rules/ide/the-request-set-is-closed.md:1`

### Backlog

- Getter and setter generation is declined by `rule:ide/a-code-action-writes-only-what-is-already-determined` (`docs/rules/ide/a-code-action-writes-only-what-is-already-determined.md`).
