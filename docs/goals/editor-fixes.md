---
looper: 1
title: Every planned quick fix, the narrowing action and the open editor gaps
summary: "Every planned quick fix and editor action is offered, every editor gap record is closed and deleted, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-lsp/src/actions.rs, docs/rules/ide/the-request-set-is-closed.md, docs/rules/ide/an-lsp-answer-is-frozen-as-an-lspt-case.md]
stages:
  - id: 1
    title: "five diagnostics carry their fix"
    status: open
    context: [crates/nvs-lsp/src/actions.rs, crates/nvs-diagnostics/src/diagnostic.rs, crates/nvs-types/src/ctor_init.rs, crates/nvs-syntax/src/parser/expr.rs, crates/nvs-syntax/src/parser/ty.rs, crates/nvs-types/src/expr/args.rs, crates/nvs-types/src/routes.rs, docs/rules/ide/a-quick-fix-is-a-diagnostics-own-suggestion.md, docs/rules/routing/a-quick-fix-writes-a-derived-path.md]
    extra:
      context:
        rules: [ide/a-quick-fix-is-a-diagnostics-own-suggestion, routing/a-quick-fix-writes-a-derived-path, classes/definite-property-initialization, statements/require-is-the-only-inclusion-construct, security/tainted-qualifier]
  - id: 2
    title: "the narrowing action writes an array's literal type"
    status: open
    context: [crates/nvs-lsp/src/actions.rs, crates/nvs-types/src/narrow.rs, docs/rules/ide/narrow-an-annotation-to-its-literal.md, docs/rules/ide/narrowest-means-narrowest-base-type.md]
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
    title: "the older `Core` reference cards are in the plain voice: the six largest modules"
    status: open
    context: [data/gaps/nvs-stdlib/reference-cards-are-not-held-to-the-plain-voice.json, docs/rules/core-api/reference-card.md]
    extra:
      context:
        rules: [core-api/reference-card]
  - id: 7
    title: "the older `Core` reference cards are in the plain voice: the next thirteen modules"
    status: open
    context: [data/gaps/nvs-stdlib/reference-cards-are-not-held-to-the-plain-voice.json, docs/rules/core-api/reference-card.md]
    extra:
      context:
        rules: [core-api/reference-card]
  - id: 8
    title: "the older `Core` reference cards are in the plain voice: every other module"
    status: open
    context: [data/gaps/nvs-stdlib/reference-cards-are-not-held-to-the-plain-voice.json, docs/rules/core-api/reference-card.md]
    extra:
      context:
        rules: [core-api/reference-card]
  - id: 9
    title: "the open document's analysis grows linearly"
    status: open
    context: [crates/nvs-lsp/src/document.rs, benches/scaling/lsp/classes.nvs, data/gaps/nvs-lsp/open-document-analysis-grows-quadratically.json]
  - id: 10
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: owners-closes
    stage: 9
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, editor-fixes]
    reach: true
    floor: false
---

# Every planned quick fix, the narrowing action and the open editor gaps

## Why

`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion` lists five quick fixes whose diagnostics
carry no suggestion yet, so the editor offers nothing on those lines. The narrowing action and its
synthesis function are designed in four rules and not built. Four editor gap records are open:
parameter hints, hover on a narrowed property, the html template region, and the open document's
near-quadratic analysis. A code lens has an empty command (`crates/nvs-lsp/src/server.rs:1200`),
and the server reads its settings only at `initialize`, so most changed settings do nothing until a
restart. Two small client items and the older reference cards finish the list. This goal closes all
of it.

## Standing decisions

**A fix is a diagnostic's own suggestion, never a second list.** Each fix in stage 1 is a
suggestion on the diagnostic (`crates/nvs-diagnostics/src/diagnostic.rs:115`), and
`crates/nvs-lsp/src/actions.rs:150` already translates every suggestion into a code action. The
laundering call is marked `alternative`, so `source.fixAll.nvs` never applies it
(`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`).

**This goal ships the fourth writing action.**
`rule:ide/a-code-action-writes-only-what-is-already-determined` names four actions. Goal
`editor-refactors` builds three, and stage 2 builds the narrowing action, so the rule names this
goal as its owner. Gate for its flip: `crates/nvs-lsp/src/generate.rs` holds all three generators
(goal `editor-refactors` stage 5 done). Until then stage 2 ships everything else and leaves that one
flip open, and the session takes stage 3.

**The narrowing round trip is defined.** The action's edit applied to a file gives a file that
passes `nvs check` with the same diagnostics as before, and the action is no longer offered on that
annotation. The `.lspt` case freezes the edit, and a unit test in `narrow.rs` applies it and checks.

**The older reference cards are rewritten here, one writing subagent per module.** A hover prints a
card, so a card in the old voice reaches every editor user. Each card is held to `AGENTS.md`
§ *Text an end user reads*. About 1,200 cards in 82 files is three sessions, so stages 6, 7 and 8
each take a fixed list of files, largest first. No `Explore` pass lists the old-voice cards first:
each writing subagent reads its whole module and rewrites only the cards in the old voice, so a
listing pass would read every card twice.

**Client tests run in the headless tier.** The setting-change test and the paste test drive the
client's handlers without an editor, under `editors/vscode/test/client/`, and run with
`npm run test:headless` in `editors/vscode`. The host tier (`npm run test:host`) is not needed. Why:
the headless tier is seconds, the host tier starts VS Code.

**Lanes.** Stages 1 and 2 share `crates/nvs-lsp/src/actions.rs`, so 2 runs after 1. Every other
stage touches its own files and runs in any order: 3 (`hints.rs`, `hover.rs`, `regions.rs`), 4
(`server.rs`, `settings.rs`, `extension.ts`), 5 (`editors/vscode/`), 6 to 8 (`crates/nvs-stdlib/`)
and 9 (`document.rs`). Stage 8 deletes the card gap record, so it runs after 6 and 7. Stages 6 to 8
can run beside the chain as a side goal.

**Trade-offs.** No program changes, and nothing runs on the request path. A suggestion on a
diagnostic costs one small allocation per diagnostic on the compile path, paid only when that
diagnostic is reported. The narrowing synthesis runs only in `nvs-lsp`
(`rule:ide/no-compile-path-calls-the-synthesis`). A setting applied without a restart keeps the
server's index warm, which usability gains from.

## Stages

### 1. five diagnostics carry their fix

**Does:** the five missing fixes of `rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion` exist as
suggestions, with an `.lspt` case each.

**Split:** an `Explore` subagent first confirms the one site that reports a `tainted` or `secret`
argument at a refusing parameter and names the laundering call, starting from `args.rs:113`. After
the five suggestions land, one writing subagent writes the five `.lspt` cases under
`tests/lsp/actions/`.

- [ ] A missing constructor property assignment (`E0409`) offers to add it (`rule:classes/definite-property-initialization`). `crates/nvs-types/src/ctor_init.rs:286`, `crates/nvs-types/src/ctor_init.rs:375`
- [ ] `include`, `include_once` and `require_once` (`E0221`) offer `require` (`rule:statements/require-is-the-only-inclusion-construct`). `crates/nvs-syntax/src/parser/expr.rs:2462`
- [ ] A `tainted` or `secret` value at a refusing sink offers the laundering call, marked `alternative` (`rule:security/tainted-qualifier`). `crates/nvs-types/src/expr/args.rs:113`
- [ ] A mis-ordered `tainted secret string` (`E0116`) offers the reordered qualifiers (`rule:security/tainted-qualifier`). `crates/nvs-syntax/src/parser/ty.rs:460`
- [ ] A `#[Route]` with no `path` (`E0747`) offers the derived one (`rule:routing/a-quick-fix-writes-a-derived-path`). `crates/nvs-types/src/routes.rs:1472`
- [ ] The fixes flow through the existing translation, and the laundering call stays out of `source.fixAll.nvs`. `crates/nvs-lsp/src/actions.rs:150`, `crates/nvs-lsp/src/actions.rs:169`
- [ ] An `.lspt` case for each fix. `tests/lsp/actions/an-include-is-offered-require.lspt:1`
- [ ] Both rules `shipped`. `data/rules/ide/a-quick-fix-is-a-diagnostics-own-suggestion.json:3`, `data/rules/routing/a-quick-fix-writes-a-derived-path.json:3`

### 2. the narrowing action writes an array's literal type

**Does:** one editor action rewrites an `array<mixed>` annotation to the type of its literal,
through one synthesis function in `nvs-types`.

**Split:** after the action lands, one writing subagent writes the `.lspt` cases under
`tests/lsp/actions/`: a nested literal, a literal in another file (not offered), a mixed element
type.

- [ ] The synthesis function, with its unit tests (`rule:ide/narrowest-means-narrowest-base-type`). `crates/nvs-types/src/narrow.rs:1`
- [ ] The guard test that no compile path calls it (`rule:ide/no-compile-path-calls-the-synthesis`). `crates/nvs-types/src/narrow.rs:1`
- [ ] The action answers from a literal in the same file or is not offered (`rule:ide/the-action-answers-from-the-literal-or-not-at-all`). `crates/nvs-lsp/src/actions.rs:150`
- [ ] It is never registered under `source.fixAll.nvs` (`rule:ide/narrowing-is-a-diff-never-a-save-time-fix`). `crates/nvs-lsp/src/actions.rs:169`
- [ ] The round trip § *Standing decisions* defines, as a unit test. `crates/nvs-types/src/narrow.rs:1`
- [ ] `.lspt` cases. `tests/lsp/actions/an-array-annotation-narrows-to-its-literal.lspt:1`
- [ ] The four narrowing rules `shipped`. `data/rules/ide/narrow-an-annotation-to-its-literal.json:3`, `data/rules/ide/narrowest-means-narrowest-base-type.json:3`, `data/rules/ide/narrowing-is-a-diff-never-a-save-time-fix.json:3`, `data/rules/ide/the-action-answers-from-the-literal-or-not-at-all.json:3`
- [ ] Gated on `crates/nvs-lsp/src/generate.rs` holding three generators: `rule:ide/a-code-action-writes-only-what-is-already-determined` `shipped`. `data/rules/ide/a-code-action-writes-only-what-is-already-determined.json:3`

### 3. parameter hints on every call, hover on a narrowed read, and an html template is a region

**Does:** three open `nvs-lsp` gap records are closed.

**Split:** after the three fixes land, one writing subagent writes the three `.lspt` cases under
`tests/lsp/hints/`, `tests/lsp/hover/` and `tests/lsp/regions/`.

- [ ] A `new`, a call through a `callable` and a call on a `mixed` receiver get parameter hints. The widening is in the table the checker records, not in the walk. `crates/nvs-lsp/src/hints.rs:455`
- [ ] Hover on a narrowed `readonly` property read shows the narrowed type (`rule:types/narrowing`). `crates/nvs-lsp/src/hover.rs:390`
- [ ] An html template's body is an HTML region, with its two kinds of hole cut out (`rule:core-classes/html-template`). `crates/nvs-lsp/src/regions.rs:268`
- [ ] An `.lspt` case for each. `tests/lsp/hints/a-constructor-call-gets-parameter-hints.lspt:1`
- [ ] The three records deleted. `data/gaps/nvs-lsp/call-the-checker-recorded-as-anything.json:1`, `data/gaps/nvs-lsp/hover-on-a-narrowed-property-read.json:1`, `data/gaps/nvs-lsp/html-templates-body-is-not-a.json:1`

### 4. a lens runs a command, and a changed setting reaches the running server

**Does:** clicking a lens does what its title says, and a changed `nvs.*` setting applies to the
running server without a restart.

- [ ] Each lens carries the command that shows what it counts: the references for a reference count, the hierarchy for a hierarchy lens (`rule:ide/five-features-are-one-reference-index`). `crates/nvs-lsp/src/server.rs:1200`
- [ ] The server handles `workspace/didChangeConfiguration` and applies `scope`, `codeLens` and `debounce` from it. `crates/nvs-lsp/src/server.rs:194`, `crates/nvs-lsp/src/settings.rs:159`
- [ ] The client sends the changed section, and restarts only for `nvs.path` and `nvs.lsp.enable`. `editors/vscode/src/extension.ts:110`, `editors/vscode/src/extension.ts:159`
- [ ] An `.lspt` case for the lens command. `tests/lsp/lens/a-reference-count-lens-opens-the-references.lspt:1`
- [ ] A headless client test for the setting change. `editors/vscode/test/client/settings.test.ts:1`

### 5. paste as doc comment, and the README says where concealment stops

**Does:** the two missing client items of their rules.

- [ ] A `DocumentPasteEditProvider` offers *Paste as doc comment* inside a `///` run, with a frozen kind id (`rule:ide/doc-comment-authoring-is-the-editors-own`). `editors/vscode/src/imports.ts:95`
- [ ] A headless test of the provider. `editors/vscode/test/client/docpaste.test.ts:1`
- [ ] `editors/vscode/README.md` lists what the concealment cannot cover, as `rule:ide/redaction-does-not-reach` names it. `editors/vscode/README.md:50`
- [ ] Both rules `shipped`. `data/rules/ide/doc-comment-authoring-is-the-editors-own.json:3`, `data/rules/ide/redaction-does-not-reach.json:3`

### 6. the older `Core` reference cards are in the plain voice: the six largest modules

**Does:** every card in six modules, about 400, reads as `rule:core-api/reference-card` asks.

**Split:** one writing subagent per module, one after another. Each reads its module's cards and
rewrites the ones in the old voice. The session reads each diff before the next subagent starts.

- [ ] `ldap/registry.rs`, 101 cards. `crates/nvs-stdlib/src/ldap/registry.rs:1`
- [ ] `time.rs`, 77 cards. `crates/nvs-stdlib/src/time.rs:1`
- [ ] `db/registry.rs`, 69 cards. `crates/nvs-stdlib/src/db/registry.rs:1`
- [ ] `arr.rs`, 59 cards. `crates/nvs-stdlib/src/arr.rs:1`
- [ ] `reflect.rs`, 48 cards. `crates/nvs-stdlib/src/reflect.rs:1`
- [ ] `io.rs`, 45 cards. `crates/nvs-stdlib/src/io.rs:1`

### 7. the older `Core` reference cards are in the plain voice: the next thirteen modules

**Does:** every card in thirteen modules, about 390, reads as `rule:core-api/reference-card` asks.

**Split:** as stage 6, one writing subagent per item.

- [ ] `str.rs` and `math.rs`, 83 cards. `crates/nvs-stdlib/src/str.rs:1`, `crates/nvs-stdlib/src/math.rs:623`
- [ ] `test.rs` and `request.rs`, 76 cards. `crates/nvs-stdlib/src/test.rs:1`, `crates/nvs-stdlib/src/request.rs:1`
- [ ] `http.rs` and `cli.rs`, 69 cards. `crates/nvs-stdlib/src/http.rs:1`, `crates/nvs-stdlib/src/cli.rs:1`
- [ ] `xml.rs`, `bigint.rs` and `uri.rs`, 76 cards. `crates/nvs-stdlib/src/xml.rs:1`, `crates/nvs-stdlib/src/bigint.rs:1`, `crates/nvs-stdlib/src/uri.rs:1`
- [ ] `router.rs`, `net.rs`, `crypto.rs` and `response.rs`, 86 cards. `crates/nvs-stdlib/src/router.rs:1`, `crates/nvs-stdlib/src/net.rs:1`, `crates/nvs-stdlib/src/crypto.rs:1`, `crates/nvs-stdlib/src/response.rs:1`

### 8. the older `Core` reference cards are in the plain voice: every other module

**Does:** every card in the remaining 63 modules, about 420, reads as `rule:core-api/reference-card`
asks, and the gap record is deleted.

**Split:** as stage 6, one writing subagent per item.

- [ ] `random.rs`, `queue.rs`, `regex.rs`, `http/stream.rs`, `process.rs`, `bytes.rs`, `test/hooks.rs`, `path.rs`. `crates/nvs-stdlib/src/random.rs:1`, `crates/nvs-stdlib/src/queue.rs:1`
- [ ] `encoding.rs`, `compress.rs`, `socket.rs`, `script.rs`, `objset.rs`, `objmap.rs`, `cache.rs`, `sse.rs`, `session.rs`. `crates/nvs-stdlib/src/objset.rs:157`, `crates/nvs-stdlib/src/objset.rs:185`
- [ ] `hash.rs`, `decimal.rs`, `ast.rs`, `uuid.rs`, `ratelimit.rs`, `jwe.rs`, `html.rs`, `env.rs`, `validate.rs`, `jwt.rs`, `heap.rs`, `storage.rs`, `os.rs`, `http/socket.rs`. `crates/nvs-stdlib/src/hash.rs:1`
- [ ] `program.rs`, `json.rs`, `csv.rs`, `config.rs`, `zip.rs`, `topic.rs`, `task.rs`, `password.rs`, `mime.rs`, `metrics.rs`, `command.rs`, `cldr.rs`, `channel.rs`, `budget.rs`, `totp.rs`, `taint.rs`, `signed_cookie.rs`, `signature.rs`, `server.rs`, `serialize.rs`, `secret.rs`, `log.rs`, `fatal.rs`, `debug.rs`, `csrf.rs`, `attributes.rs`, `signal.rs`, `out.rs`, `mail.rs`, `cap.rs`, `registry.rs`, `db/stream.rs`. `crates/nvs-stdlib/src/program.rs:1`
- [ ] The stdlib's card tests still pass in the group's verify. `crates/nvs-stdlib/src/registry.rs:5105`
- [ ] The record deleted. `data/gaps/nvs-stdlib/reference-cards-are-not-held-to-the-plain-voice.json:1`

### 9. the open document's analysis grows linearly

**Does:** the server's analysis of one open document is linear in its size, as `nvs check` already
is.

**Split:** an `Explore` subagent first lists every pass `analyse_current` runs over an open document,
as `file:line`, and which of them walks the whole document per declaration.

- [ ] The quadratic pass found and named in the gap's place, with the ladder's slope before. `crates/nvs-lsp/src/document.rs:843`, `benches/scaling/lsp/classes.nvs:1`
- [ ] The pass made linear. `crates/nvs-lsp/src/document.rs:647`
- [ ] `bun nv scaling` passes `benches/scaling/lsp/classes.nvs`, which already declares `expect linear`. `benches/scaling/lsp/classes.nvs:6`
- [ ] The record deleted. `data/gaps/nvs-lsp/open-document-analysis-grows-quadratically.json:1`
- [ ] `bun nv owners --closes editor-fixes` passes. `data/rules/ide/a-quick-fix-is-a-diagnostics-own-suggestion.json:3`

### 10. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

No stage is started. The anchors above were checked against the tree. Stage 2's last rule flip
waits on goal `editor-refactors` (`crates/nvs-lsp/src/generate.rs`). Every other stage has no gate.

### Next

**Stage 1: five diagnostics carry their fix**, one file set: `crates/nvs-types/src/ctor_init.rs`,
`crates/nvs-syntax/src/parser/expr.rs`, `crates/nvs-syntax/src/parser/ty.rs`,
`crates/nvs-types/src/expr/args.rs`, `crates/nvs-types/src/routes.rs`, `crates/nvs-lsp/src/actions.rs`,
`tests/lsp/actions/`. No gate.

- [ ] The constructor assignment (`rule:classes/definite-property-initialization`). `crates/nvs-types/src/ctor_init.rs:286`, `crates/nvs-types/src/ctor_init.rs:375`
- [ ] `include` to `require` (`rule:statements/require-is-the-only-inclusion-construct`). `crates/nvs-syntax/src/parser/expr.rs:2462`
- [ ] The laundering call, `alternative` (`rule:security/tainted-qualifier`). `crates/nvs-types/src/expr/args.rs:113`
- [ ] The reordered qualifiers (`rule:security/tainted-qualifier`). `crates/nvs-syntax/src/parser/ty.rs:460`
- [ ] The derived `#[Route]` path (`rule:routing/a-quick-fix-writes-a-derived-path`). `crates/nvs-types/src/routes.rs:1472`
- [ ] The translation, the laundering call out of fix-all. `crates/nvs-lsp/src/actions.rs:150`, `crates/nvs-lsp/src/actions.rs:169`
- [ ] An `.lspt` case each. `tests/lsp/actions/an-include-is-offered-require.lspt:1`
- [ ] Both rules `shipped`. `data/rules/ide/a-quick-fix-is-a-diagnostics-own-suggestion.json:3`, `data/rules/routing/a-quick-fix-writes-a-derived-path.json:3`

### Backlog

- The copy half of the import-carrying paste has no proof in a real editor, because nothing makes the editor fire a real copy (`editors/vscode/src/imports.ts`, § *Known gaps*).
- `semanticTokenScopes` needs no new entry: every token type is in LSP's standard legend, and only the two modifiers are mapped (`docs/rules/ide/novis-ships-names-not-colours.md`).
