---
looper: 1
title: Every planned quick fix, the narrowing action and the open editor gaps
summary: "Every planned quick fix and editor action is offered, every editor gap record is closed and deleted, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-lsp/src/actions.rs, docs/rules/ide/the-request-set-is-closed.md, docs/rules/ide/an-lsp-answer-is-frozen-as-an-lspt-case.md]
stages:
  - id: 1
    title: "five diagnostics carry their fix"
    status: open
    context: [crates/nvs-lsp/src/actions.rs, crates/nvs-diagnostics/src/lib.rs, crates/nvs-types/src/routes.rs, docs/rules/ide/a-quick-fix-is-a-diagnostics-own-suggestion.md, docs/rules/routing/a-quick-fix-writes-a-derived-path.md]
    extra:
      context:
        rules: [ide/a-quick-fix-is-a-diagnostics-own-suggestion, routing/a-quick-fix-writes-a-derived-path, classes/definite-property-initialization, statements/require-is-the-only-inclusion-construct, security/tainted-qualifier]
  - id: 2
    title: "the narrowing action writes an array's literal type"
    status: open
    context: [crates/nvs-lsp/src/actions.rs, docs/rules/ide/narrow-an-annotation-to-its-literal.md, docs/rules/ide/narrowest-means-narrowest-base-type.md, docs/rules/ide/narrowing-is-a-diff-never-a-save-time-fix.md, docs/rules/ide/the-action-answers-from-the-literal-or-not-at-all.md, docs/rules/ide/no-compile-path-calls-the-synthesis.md]
    extra:
      context:
        rules: [ide/narrow-an-annotation-to-its-literal, ide/narrowest-means-narrowest-base-type, ide/narrowing-is-a-diff-never-a-save-time-fix, ide/the-action-answers-from-the-literal-or-not-at-all, ide/no-compile-path-calls-the-synthesis, ide/a-code-action-writes-only-what-is-already-determined]
  - id: 3
    title: "parameter hints on every call, hover on a narrowed read, and an html template is a region"
    status: open
    context: [crates/nvs-lsp/src/hints.rs, crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/regions.rs, data/gaps/nvs-lsp/call-the-checker-recorded-as-anything.json, data/gaps/nvs-lsp/hover-on-a-narrowed-property-read.json, data/gaps/nvs-lsp/html-templates-body-is-not-a.json]
    extra:
      context:
        rules: [types/narrowing, core-classes/html-template, ide/a-template-region-gets-the-editors-services-and-formatter]
  - id: 4
    title: "a lens runs a command, and a changed setting reaches the running server"
    status: open
    context: [crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/settings.rs, editors/vscode/src/extension.ts]
    extra:
      context:
        rules: [ide/five-features-are-one-reference-index, ide/contributions-are-frozen-and-only-ever-added]
  - id: 5
    title: "paste as doc comment, and the README says where concealment stops"
    status: open
    context: [editors/vscode/src/imports.ts, editors/vscode/README.md, docs/rules/ide/doc-comment-authoring-is-the-editors-own.md, docs/rules/ide/redaction-does-not-reach.md]
    extra:
      context:
        rules: [ide/doc-comment-authoring-is-the-editors-own, ide/redaction-does-not-reach]
  - id: 6
    title: "the older `Core` reference cards are in the plain voice"
    status: open
    context: [crates/nvs-stdlib/src/registry.rs, data/gaps/nvs-stdlib/reference-cards-are-not-held-to-the-plain-voice.json, docs/rules/core-api/reference-card.md]
    extra:
      context:
        rules: [core-api/reference-card]
checks:
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, editor-fixes]
    reach: true
    floor: false
---

# Every planned quick fix, the narrowing action and the open editor gaps

## Why

`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion` lists five quick fixes whose diagnostics
carry no suggestion yet, so the editor offers nothing on those lines. The narrowing action and its
synthesis function are designed in four rules and not built. Three editor gap records are open:
parameter hints, hover on a narrowed property, and the html template region. A code lens has an
empty command (`crates/nvs-lsp/src/server.rs:1204`), and the server reads its settings only at
`initialize`, so most changed settings do nothing until a restart. Two small client items and the
older reference cards finish the list. This goal closes all of it.

## Standing decisions

**A fix is a diagnostic's own suggestion, never a second list.** Each fix in stage 1 is a
suggestion on the diagnostic, and `crates/nvs-lsp/src/actions.rs:150` already translates every
suggestion into a code action. The laundering call is marked `alternative`, so `source.fixAll.nvs`
never applies it (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`).

**This goal ships the fourth writing action.** The planning session's call.
`rule:ide/a-code-action-writes-only-what-is-already-determined` names four actions. Goal
`editor-refactors` builds three, and stage 2 builds the narrowing action, so the rule names this
goal as its owner.

**The older reference cards are rewritten here.** The planning session's call. A hover prints a
card, so a card in the old voice reaches every editor user. The gap record says this goal rewrites
them. Each card is held to `AGENTS.md` § *Text an end user reads*.

**Trade-offs.** No program changes, and nothing runs on the request path. A suggestion on a
diagnostic costs one small allocation per diagnostic on the compile path, paid only when that
diagnostic is reported. The narrowing synthesis runs only in `nvs-lsp`
(`rule:ide/no-compile-path-calls-the-synthesis`). A setting applied without a restart keeps the
server's index warm, which usability gains from.

## Stages

### 1. five diagnostics carry their fix

**Does:** the five missing fixes of `rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion` exist as
suggestions, with an `.lspt` case each.

- [ ] A missing constructor property assignment offers to add it (`rule:classes/definite-property-initialization`).
- [ ] `include`, `include_once` and `require_once` offer `require` (`rule:statements/require-is-the-only-inclusion-construct`). `crates/nvs-diagnostics/src/lib.rs:618`
- [ ] A `tainted` or `secret` value at a refusing sink offers the laundering call, marked `alternative`, and a mis-ordered `tainted secret string` offers the reordered qualifiers (`rule:security/tainted-qualifier`).
- [ ] A `#[Route]` with no `path` offers the derived one (`rule:routing/a-quick-fix-writes-a-derived-path`). `crates/nvs-diagnostics/src/lib.rs:3955`, `crates/nvs-types/src/routes.rs:1`
- [ ] The fixes flow through the existing translation, and both rules are `shipped`. `crates/nvs-lsp/src/actions.rs:150`, `crates/nvs-lsp/src/actions.rs:208`

### 2. the narrowing action writes an array's literal type

**Does:** one editor action rewrites an `array<mixed>` annotation to the type of its literal,
through one synthesis function in `nvs-types`.

- [ ] The synthesis function in `nvs-types`, with its unit tests (`rule:ide/narrowest-means-narrowest-base-type`), and the guard test that no compile path calls it (`rule:ide/no-compile-path-calls-the-synthesis`).
- [ ] The action answers from a literal in the same file or is not offered (`rule:ide/the-action-answers-from-the-literal-or-not-at-all`). `crates/nvs-lsp/src/actions.rs:150`
- [ ] It is never registered under `source.fixAll.nvs` (`rule:ide/narrowing-is-a-diff-never-a-save-time-fix`).
- [ ] The round trip and the `.lspt` cases, and the four narrowing rules and `rule:ide/a-code-action-writes-only-what-is-already-determined` are `shipped`.

### 3. parameter hints on every call, hover on a narrowed read, and an html template is a region

**Does:** the three open `nvs-lsp` gap records are closed.

- [ ] A `new`, a call through a `callable` and a call on a `mixed` receiver get parameter hints. The widening is in the table the checker records, not in the walk. `crates/nvs-lsp/src/hints.rs:460`, `data/gaps/nvs-lsp/call-the-checker-recorded-as-anything.json:1`
- [ ] Hover on a narrowed `readonly` property read shows the narrowed type (`rule:types/narrowing`). `crates/nvs-lsp/src/hover.rs:392`
- [ ] An html template's body is an HTML region, with its two kinds of hole cut out (`rule:core-classes/html-template`). `crates/nvs-lsp/src/regions.rs:207`
- [ ] An `.lspt` case for each, and the three records are deleted.

### 4. a lens runs a command, and a changed setting reaches the running server

**Does:** clicking a lens does what its title says, and a changed `nvs.*` setting applies to the
running server without a restart.

- [ ] Each lens carries the command that shows what it counts: the references for a reference count, the hierarchy for a hierarchy lens (`rule:ide/five-features-are-one-reference-index`). `crates/nvs-lsp/src/server.rs:1204`
- [ ] The server handles `workspace/didChangeConfiguration` and applies `scope`, `codeLens` and `debounce` from it. `crates/nvs-lsp/src/server.rs:194`
- [ ] The client sends the changed section, and restarts only for `nvs.path` and `nvs.lsp.enable`. `editors/vscode/src/extension.ts:159`, `editors/vscode/src/extension.ts:434`
- [ ] An `.lspt` case for the lens command, and a client test for the setting change.

### 5. paste as doc comment, and the README says where concealment stops

**Does:** the two missing client items of their rules.

- [ ] A `DocumentPasteEditProvider` offers *Paste as doc comment* inside a `///` run, with a frozen kind id (`rule:ide/doc-comment-authoring-is-the-editors-own`). `editors/vscode/src/imports.ts:95`
- [ ] `editors/vscode/README.md` lists what the concealment cannot cover, as `rule:ide/redaction-does-not-reach` names it. `editors/vscode/README.md:50`
- [ ] Both rules are `shipped`.

### 6. the older `Core` reference cards are in the plain voice

**Does:** every `Core` reference card reads as `rule:core-api/reference-card` asks.

- [ ] Each card in the old voice is rewritten, for example `crates/nvs-stdlib/src/objset.rs:157`, `crates/nvs-stdlib/src/objset.rs:185` and `crates/nvs-stdlib/src/math.rs:623`.
- [ ] The stdlib's card tests still pass, and the record is deleted. `data/gaps/nvs-stdlib/reference-cards-are-not-held-to-the-plain-voice.json:1`
- [ ] `bun nv owners --closes editor-fixes` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written.

### Next

Stage 1, the five fixes, on `crates/nvs-lsp/src/actions.rs`, `crates/nvs-diagnostics/src/lib.rs`
and the checker modules that raise each diagnostic:
- `include` to `require` (`rule:statements/require-is-the-only-inclusion-construct`). `crates/nvs-diagnostics/src/lib.rs:618`
- the derived `#[Route]` path (`rule:routing/a-quick-fix-writes-a-derived-path`). `crates/nvs-diagnostics/src/lib.rs:3955`
- the laundering call and the reordered qualifiers (`rule:security/tainted-qualifier`). `crates/nvs-lsp/src/actions.rs:150`

### Backlog

- The copy half of the import-carrying paste has no proof in a real editor, because nothing makes the editor fire a real copy (`editors/vscode/src/imports.ts`, § *Known gaps*).
- `semanticTokenScopes` needs no new entry: every token type is in LSP's standard legend, and only the two modifiers are mapped (`docs/rules/ide/novis-ships-names-not-colours.md`).
