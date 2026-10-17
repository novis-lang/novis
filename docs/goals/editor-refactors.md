---
looper: 1
title: The editor renames, finds, imports and generates
summary: "Workspace rename, workspace symbol search, organize imports, the two extractions and the three generators answer as their rules say, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-lsp/src/capabilities.rs, crates/nvs-lsp/src/index.rs, docs/rules/ide/the-request-set-is-closed.md, docs/rules/ide/an-lsp-answer-is-frozen-as-an-lspt-case.md]
stages:
  - id: 1
    title: "rename across the workspace, and a file rename updates `require` paths"
    status: open
    context: [crates/nvs-lsp/src/capabilities.rs, crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/server.rs, docs/rules/ide/no-refactoring-introduces-an-alias.md, docs/rules/ide/five-features-are-one-reference-index.md]
    extra:
      context:
        rules: [ide/no-refactoring-introduces-an-alias, ide/five-features-are-one-reference-index, statements/require-is-the-only-inclusion-construct]
  - id: 2
    title: "workspace symbol search, and organize imports"
    status: open
    context: [crates/nvs-lsp/src/capabilities.rs, crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/imports.rs, docs/rules/ide/no-refactoring-introduces-an-alias.md]
    extra:
      context:
        rules: [ide/no-refactoring-introduces-an-alias, statements/nothing-gets-a-second-name]
  - id: 3
    title: "extract to variable and extract to method"
    status: open
    context: [crates/nvs-lsp/src/actions.rs, crates/nvs-lsp/src/server.rs, docs/rules/ide/no-refactoring-introduces-an-alias.md]
    extra:
      context:
        rules: [ide/no-refactoring-introduces-an-alias, ide/a-code-action-writes-only-what-is-already-determined]
  - id: 4
    title: "implement missing members, override a method, and declare the function you called"
    status: open
    context: [crates/nvs-lsp/src/actions.rs, crates/nvs-lsp/src/completion.rs, docs/rules/ide/a-code-action-writes-only-what-is-already-determined.md]
    extra:
      context:
        rules: [ide/a-code-action-writes-only-what-is-already-determined, statements/inout-is-the-by-reference-spelling]
checks:
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, editor-refactors]
    reach: true
    floor: false
extra:
  milestone: M10
---

# The editor renames, finds, imports and generates

## Why

The language server answers references, highlights, lenses, signature help and inlay hints from
one reference index, but it offers none of the refactorings
`rule:ide/no-refactoring-introduces-an-alias` names. `server_capabilities` leaves `rename` and
`workspaceSymbol` unset (`crates/nvs-lsp/src/capabilities.rs:158`). There is no organize imports, no
extraction, and none of the three generators of
`rule:ide/a-code-action-writes-only-what-is-already-determined`. This goal builds them, in the order
each one's dependency lands.

## Standing decisions

**Member resolution is completeness-holes' stage 10, not this goal's.** The planning session's
outline put completion through the class graph here. Goal `completeness-holes` built it: completion
walks a class's parents and interfaces and filters each member by visibility. Stage 4's generators
read the members that walk resolves (`crates/nvs-lsp/src/completion.rs:2928`).

**The narrowing action is goal `editor-fixes`' work.** The planning session's call.
`rule:ide/a-code-action-writes-only-what-is-already-determined` names four writing actions. This
goal builds three, so the rule ships with the fourth and names goal `editor-fixes` as its owner.

**No refactoring writes an alias.** `rule:ide/no-refactoring-introduces-an-alias` binds every stage.
A rename that would make two names clash refuses with a message. It never writes `use ... as`.

**Trade-offs.** No program changes, and nothing runs on the request path. Rename and symbol search
read the reference index the server already holds, so they add no store. The index answers
per-request work in the editor, and its memory does not change. Each action is one more answer in
the closed request set, which `rule:ide/the-request-set-is-closed` lists, so the protocol surface
grows by the requests named here and no others.

## Stages

### 1. rename across the workspace, and a file rename updates `require` paths

**Does:** `textDocument/rename` with `prepareRename` renames a symbol at every occurrence the index
holds, and `workspace/willRenameFiles` rewrites each `require` path that names a renamed file.

- [ ] `prepareRename` refuses a position that is no renamable symbol, a `Core` name among them. `crates/nvs-lsp/src/capabilities.rs:158`
- [ ] `rename` edits every occurrence from the index, and refuses a new name that clashes in any file it touches (`rule:ide/no-refactoring-introduces-an-alias`). `crates/nvs-lsp/src/index.rs:623`
- [ ] `willRenameFiles` returns the `require` path edits (`rule:statements/require-is-the-only-inclusion-construct`).
- [ ] `.lspt` cases for each, and `rule:ide/the-request-set-is-closed` lists the new requests.

### 2. workspace symbol search, and organize imports

**Does:** `workspace/symbol` finds a declaration in any indexed file, and organize imports reorders
the `use` block and removes unused lines.

- [ ] `workspace/symbol` answers from the index. `crates/nvs-lsp/src/capabilities.rs:158`, `crates/nvs-lsp/src/index.rs:378`
- [ ] Organize imports is a `source.organizeImports` action built on the edit machinery of `nvs/importEdits`. It only reorders and removes (`rule:ide/no-refactoring-introduces-an-alias`). `crates/nvs-lsp/src/imports.rs:67`, `crates/nvs-lsp/src/imports.rs:241`
- [ ] Its order is the order `rule:tooling/fmt-sorts-the-use-block` gives, so a format after it changes nothing.
- [ ] `.lspt` cases for each.

### 3. extract to variable and extract to method

**Does:** two `refactor.extract` actions over a selection.

- [ ] Extract to variable writes a typed local declaration from the type the checker recorded for the selection, and replaces the selection with the variable. `crates/nvs-lsp/src/actions.rs:150`
- [ ] Extract to method writes a method whose parameters are the locals the selection reads, typed as the checker recorded them, and whose return type is the selection's. What it does with a selection that writes a local or returns early is settled in `rule:ide/no-refactoring-introduces-an-alias` before the code.
- [ ] Neither action is offered where a type is not fully known (`rule:ide/a-code-action-writes-only-what-is-already-determined`).
- [ ] `.lspt` cases for each, and `rule:ide/no-refactoring-introduces-an-alias` is `shipped` once stages 1 to 3 are done.

### 4. implement missing members, override a method, and declare the function you called

**Does:** the three generators of `rule:ide/a-code-action-writes-only-what-is-already-determined`,
each writing only what the type system already determined.

- [ ] Implement missing members inserts every missing method and property with its declared signature, `inout` and qualifiers kept, and a body that throws. `crates/nvs-lsp/src/actions.rs:150`
- [ ] Override a method offers the hierarchy's overridable members and inserts the chosen signatures with a call to the base. `crates/nvs-lsp/src/completion.rs:2918`
- [ ] Declare the function you called acts on the unresolved-call diagnostic and writes the argument types and the return type the diagnostic computed. Novis has no free function (`E0215`, `E0320`), so the rule first says that it declares a method on the receiver's class. `crates/nvs-diagnostics/src/lib.rs:580`, `crates/nvs-diagnostics/src/lib.rs:1128`
- [ ] Each one applied twice is a no-op, and a class with the first one applied passes `nvs check`. `.lspt` cases for each.
- [ ] `bun nv owners --closes editor-refactors` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written.

### Next

Stage 1, rename, on `crates/nvs-lsp/src/capabilities.rs`, `crates/nvs-lsp/src/index.rs` and
`crates/nvs-lsp/src/server.rs`:
- `prepareRename` and `rename` from the index (`rule:ide/no-refactoring-introduces-an-alias`). `crates/nvs-lsp/src/index.rs:623`
- `willRenameFiles` for `require` paths. `crates/nvs-lsp/src/capabilities.rs:158`

### Backlog

- Getter and setter generation is declined by `rule:ide/a-code-action-writes-only-what-is-already-determined` (`docs/rules/ide/a-code-action-writes-only-what-is-already-determined.md`).
