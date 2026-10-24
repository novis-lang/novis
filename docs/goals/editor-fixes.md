---
looper: 1
title: Every planned quick fix, the narrowing action and the open editor gaps
summary: "Every planned quick fix and editor action is offered, every editor gap record is closed and deleted, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-lsp/src/actions.rs, docs/rules/ide/the-request-set-is-closed.md, docs/rules/ide/an-lsp-answer-is-frozen-as-an-lspt-case.md]
stages:
  - id: 1
    title: "five diagnostics carry their fix"
    status: done
    context: [crates/nvs-lsp/src/actions.rs, crates/nvs-diagnostics/src/diagnostic.rs, crates/nvs-types/src/ctor_init.rs, crates/nvs-syntax/src/parser/expr.rs, crates/nvs-syntax/src/parser/ty.rs, crates/nvs-types/src/expr/args.rs, crates/nvs-types/src/routes.rs, docs/rules/ide/a-quick-fix-is-a-diagnostics-own-suggestion.md, docs/rules/routing/a-quick-fix-writes-a-derived-path.md]
    extra:
      context:
        rules: [ide/a-quick-fix-is-a-diagnostics-own-suggestion, routing/a-quick-fix-writes-a-derived-path, classes/definite-property-initialization, statements/require-is-the-only-inclusion-construct, security/tainted-qualifier]
  - id: 2
    title: "the narrowing action writes an array's literal type"
    status: done
    context: [crates/nvs-lsp/src/actions.rs, crates/nvs-types/src/narrow.rs, docs/rules/ide/narrow-an-annotation-to-its-literal.md, docs/rules/ide/narrowest-means-narrowest-base-type.md]
    extra:
      context:
        rules: [ide/narrow-an-annotation-to-its-literal, ide/narrowest-means-narrowest-base-type, ide/narrowing-is-a-diff-never-a-save-time-fix, ide/the-action-answers-from-the-literal-or-not-at-all, ide/no-compile-path-calls-the-synthesis, ide/a-code-action-writes-only-what-is-already-determined]
  - id: 3
    title: "parameter hints on every call, hover on a narrowed read, and an html template is a region"
    status: done
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
annotation. The `.lspt` case freezes the edit. The test that applies it is
`a_narrowed_annotation_checks_as_before_and_is_not_offered_again` in
`crates/nvs-lsp/tests/actions.rs`, because only `nvs-lsp` can analyse an edited document.

**The narrowing reads `var`'s synthesis, and writes no second one.** The shipped
`rule:ide/no-compile-path-calls-the-synthesis` already names `synthesize_array_literal` as the one
synthesis. So the action analyses the document again with `var` in place of the annotation, and
reads the type `var_array_literal` records at the literal (`ExprTypeTable::literal_type`), a refused
union included. `crates/nvs-types/src/narrow.rs` only widens, declines and prints. The action is
offered only when the edited document reports the diagnostics it reports now, so it never breaks a
later write. It covers local declarations; a property's default and a parameter's are not offered.
A union is printed with its members in text order, since the interner's order differs between
documents.

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

**A fix is offered only where the diagnostic knows every word of it.** The constructor assignment is
`$this->name = $name;`, offered only where a parameter that is not promoted has the property's name.
The laundering call is `Core\Http::allowUrl(<argument>)`, the one launderer that takes the refused
value as its only argument. `Core\Ldap\Dn::of` needs an attribute name, and `Core\Secret::reveal` and
`Core\Taint::assertTrusted` need a reason only the author can give, so a `secret` value and a DN get
no fix. The plain sink mismatch names no launderer on purpose (`crates/nvs-types/src/expr/args.rs`,
`check_shape_field`'s doc), so it gets none either.

**A call names its parameters only where the program does.** A method call, a static call,
`$cls::f(...)`, `new` and `new $cls(...)` read the resolved target's `param_names`, a constructor's
for a `new`. A call through a `callable` and a call on a `mixed` receiver get no parameter hint:
`rule:types/callable-signature` forbids names in a callable type, and a `mixed` receiver's method is
found only when it runs (`rule:types/erased-member-access`). A name drawn there would be one the
server picked, which `crates/nvs-lsp/src/hints.rs`'s first decision refuses, so no field was added to
`ExprInfo::CallThroughSignature` or `ExprInfo::ErasedCall`. The module doc of `hints.rs` is the home.

## Stages

### 1. five diagnostics carry their fix

**Does:** the five missing fixes of `rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion` exist as
suggestions, with an `.lspt` case each.

**Split:** an `Explore` subagent first confirms the one site that reports a `tainted` or `secret`
argument at a refusing parameter and names the laundering call, starting from `args.rs:113`. After
the five suggestions land, one writing subagent writes the five `.lspt` cases under
`tests/lsp/actions/`.

- [x] A missing constructor property assignment (`E0409`) offers to add it (`rule:classes/definite-property-initialization`). `crates/nvs-types/src/ctor_init.rs:286`, `crates/nvs-types/src/ctor_init.rs:375`
- [x] `include`, `include_once` and `require_once` (`E0221`) offer `require` (`rule:statements/require-is-the-only-inclusion-construct`). `crates/nvs-syntax/src/parser/expr.rs:2462`
- [x] A `tainted` or `secret` value at a refusing sink offers the laundering call, marked `alternative` (`rule:security/tainted-qualifier`). `crates/nvs-types/src/expr/args.rs:113`
- [x] A mis-ordered `tainted secret string` (`E0116`) offers the reordered qualifiers (`rule:security/tainted-qualifier`). `crates/nvs-syntax/src/parser/ty.rs:460`
- [x] A `#[Route]` with no `path` (`E0747`) offers the derived one (`rule:routing/a-quick-fix-writes-a-derived-path`). `crates/nvs-types/src/routes.rs:1472`
- [x] The fixes flow through the existing translation, and the laundering call stays out of `source.fixAll.nvs`. `crates/nvs-lsp/src/actions.rs:150`, `crates/nvs-lsp/src/actions.rs:169`
- [x] An `.lspt` case for each fix. `tests/lsp/actions/an-include-is-offered-require.lspt:1`
- [x] Both rules `shipped`. `data/rules/ide/a-quick-fix-is-a-diagnostics-own-suggestion.json:3`, `data/rules/routing/a-quick-fix-writes-a-derived-path.json:3`

### 2. the narrowing action writes an array's literal type

**Does:** one editor action rewrites an `array<mixed>` annotation to the type of its literal,
through one synthesis function in `nvs-types`.

**Split:** after the action lands, one writing subagent writes the `.lspt` cases under
`tests/lsp/actions/`: a nested literal, a literal in another file (not offered), a mixed element
type.

- [x] The synthesis function, with its unit tests (`rule:ide/narrowest-means-narrowest-base-type`). `crates/nvs-types/src/narrow.rs:1`
  - [x] `var_array_literal` records the synthesis outcome at the literal's span (`ExprTypeTable::literal_type`).
  - [x] `narrow::narrowest` widens each element to its base and declines `mixed`, an unwritable type and depth past 32.
  - [x] `narrow::written` renders it, `?T` for a two-member union with `null`.
  - [x] The seven unit tests the rule lists.
- [x] The guard test that no compile path calls it (`rule:ide/no-compile-path-calls-the-synthesis`). `crates/nvs-types/src/narrow.rs:1`
- [x] The action answers from a literal in the same file or is not offered (`rule:ide/the-action-answers-from-the-literal-or-not-at-all`). `crates/nvs-lsp/src/actions.rs:150`
- [x] It is never registered under `source.fixAll.nvs` (`rule:ide/narrowing-is-a-diff-never-a-save-time-fix`). `crates/nvs-lsp/src/actions.rs:169`
- [x] The round trip § *Standing decisions* defines, as a unit test. `crates/nvs-types/src/narrow.rs:1`
- [x] `.lspt` cases. `tests/lsp/actions/an-array-annotation-narrows-to-its-literal.lspt:1`
- [x] The four narrowing rules `shipped`. `data/rules/ide/narrow-an-annotation-to-its-literal.json:3`, `data/rules/ide/narrowest-means-narrowest-base-type.json:3`, `data/rules/ide/narrowing-is-a-diff-never-a-save-time-fix.json:3`, `data/rules/ide/the-action-answers-from-the-literal-or-not-at-all.json:3`
- [x] Gated on `crates/nvs-lsp/src/generate.rs` holding three generators: `rule:ide/a-code-action-writes-only-what-is-already-determined` `shipped`. `data/rules/ide/a-code-action-writes-only-what-is-already-determined.json:3`

### 3. parameter hints on every call, hover on a narrowed read, and an html template is a region

**Does:** three open `nvs-lsp` gap records are closed.

**Split:** after the three fixes land, one writing subagent writes the three `.lspt` cases under
`tests/lsp/hints/`, `tests/lsp/hover/` and `tests/lsp/regions/`.

- [x] A `new`, a call through a `callable` and a call on a `mixed` receiver get parameter hints. The widening is in the table the checker records, not in the walk. A `callable` call and a `mixed` receiver get none, by the standing decision *A call names its parameters only where the program does*. `crates/nvs-lsp/src/hints.rs:455`
- [x] Hover on a narrowed `readonly` property read shows the narrowed type (`rule:types/narrowing`). `crates/nvs-lsp/src/hover.rs:390`
- [x] An html template's body is an HTML region, with its two kinds of hole cut out (`rule:core-classes/html-template`). `crates/nvs-lsp/src/regions.rs:268`
- [x] An `.lspt` case for each. `tests/lsp/hints/a-constructor-call-gets-parameter-hints.lspt:1`
  - [x] The three cases, and nine more so `HtmlTemplate` is reached by every cursor-taking request.
- [x] The three records deleted. `data/gaps/nvs-lsp/call-the-checker-recorded-as-anything.json:1`, `data/gaps/nvs-lsp/hover-on-a-narrowed-property-read.json:1`, `data/gaps/nvs-lsp/html-templates-body-is-not-a.json:1`

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

Stages 1, 2 and 3 are done. Stage 3: `hints.rs`'s `resolved_call` reads the parameter names of a
`new` (`ExprInfo::New`/`NewDynamic`'s `ctor`) and a `$cls::f(...)` (`ClassRefCall`) as well as a
method call. A `callable` call and a `mixed` receiver get none, by the new standing decision. Hover
on a `readonly` property read shows `ExprInfo::Property`'s `narrowed` type. `regions.rs`'s
`for_source` reports an html template's body runs as HTML regions, with a stack of open delimiters
so a string inside a hole is not one. The three gap records are deleted. Twelve new `.lspt` cases
(suite: 550 passed): one each under `hints/`, `hover/` and `regions/`, and nine that reach the new
`HtmlTemplate` coverage column from every cursor-taking request, so
`every_request_answers_every_construct` stays green. The hints case passes strings only, so no
`Int` column under `inlayHint` was added.

Stage 1: five diagnostics carry their fix as a suggestion, each frozen by
an `.lspt` case under `tests/lsp/actions/` (`E0409`, `E0221`, `E0116`, `E0747`, a `tainted` URL's
`E0401`). Stage 2: *Narrow the type* is a `refactor.rewrite` action in
`crates/nvs-lsp/src/narrow.rs`, appended to quick-fix answers only (`crates/nvs-lsp/src/actions.rs`).
It reads `ExprTypeTable::literal_type`, which `var_array_literal` in `crates/nvs-types/src/locals.rs`
now records, and `crates/nvs-types/src/narrow.rs` widens and prints the type. The new standing
decision says how. Five rules are `shipped`. Tests: `narrow::tests` (8, green),
`crates/nvs-lsp/tests/actions.rs` (green), and five narrowing `.lspt` cases (green). The old case
`a-fix-the-checker-would-compute-is-offered-by-nothing.lspt` is now
`an-array-annotation-narrows-to-its-literal.lspt`.

Tests likely to go red in the gate stage. Stage 2 adds a `refactor.rewrite` action on any
`array<mixed> $x = [...]` annotation, so an `.lspt` or a test that asks `codeAction` with the cursor
on one now gets it. `bun nv lsp-test tests/lsp/ --coverage` may count a new action. Stage 1's, each
printing the new `= suggestion` line or checking a diagnostic's suggestions:
- `crates/nvs-syntax/src/parser/tests/ty.rs` (the `E0116` case) and any parser test that counts
  suggestions.
- `crates/nvs-types/tests/routes.rs` (`E0747`) and the `E0409` unit tests in `ctor_init.rs`.
- Any `.nvst` whose `--EXPECTF-ERROR--` writes an `E0409`, `E0221`, `E0116` or `E0747` block
  without a `%A` after it. `tests/conformance/core/every-client-member-refuses-a-tainted-url.nvst`
  is already updated, and the two `reject/*require*` cases pass.
- A `nvs check --json` snapshot that holds one of these codes.

**The nightly run's red is not owed by this tree.** Run 39922764699 ran at `7b6ef53ee`. Its
`cost guards / macos-aarch64` soak leak was 2280 bytes per connection, and commit `071663316`
(`fix(runtime): a spawned tree's start-up bytes leave the tree that spawned it`) fixes it. That
commit is in `HEAD` and not in the run. The soak passes at `HEAD` on Windows under `--release`. It
has never run on Linux, so the next nightly is its first run there. `env-secret-proofs` exits 0 at
`HEAD` (`76ff1b68d`, run again this session, and its log reads `exit 0`), so the sweep's red is
stale. The uncommitted rustfmt change in `crates/nvs-runtime/src/capability.rs` is not this goal's,
and is left in the tree.

### Next

**Stage 4: a lens runs a command, and a changed setting reaches the running server.** Its files are
`crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/settings.rs` and
`editors/vscode/src/extension.ts`. Nothing handles `didChangeConfiguration` yet.

- [ ] Each lens carries the command that shows what it counts. `crates/nvs-lsp/src/server.rs:1461` (`code_lens`, the `CodeLens` at 1488), `crates/nvs-lsp/src/render.rs:697`
- [ ] The server handles `workspace/didChangeConfiguration` and applies `scope`, `codeLens` and `debounce`. `crates/nvs-lsp/src/server.rs:194`, `crates/nvs-lsp/src/settings.rs:159`
- [ ] The client sends the changed section, and restarts only for `nvs.path` and `nvs.lsp.enable`. `editors/vscode/src/extension.ts:110`, `editors/vscode/src/extension.ts:159`
- [ ] An `.lspt` case for the lens command. `tests/lsp/lens/a-reference-count-lens-opens-the-references.lspt:1`
- [ ] A headless client test for the setting change. `editors/vscode/test/client/settings.test.ts:1`

A new capability or handled notification changes what `crates/nvs-lsp/tests/handshake.rs` holds.

### Backlog

- The narrowing action covers local declarations only. A property's default, a parameter's and a
  declaration inside a closure are not offered (`crates/nvs-lsp/src/narrow.rs`, `stmt_at`).
- An `E0747` fix on a parameter whose type does not resolve writes a capture for it, because
  `converts_from_segment` accepts an unresolved type (`derived_path` in
  `crates/nvs-types/src/routes.rs`). The `E0303` beside it is the real error.
- References and document highlight answer `none` on a local variable, and so does a variable in
  a template hole, while rename and prepareRename on the same variable work
  (`tests/lsp/rename/a-variable-inside-an-html-template-hole-is-renamed-with-its-declaration.lspt`).
  The existing `definition/a-plain-variable-read-answers-none.lspt` freezes that as intended.
- There is no `Core\Http\Request` class. A route case that wants a service parameter declares its own class.

- The copy half of the import-carrying paste has no proof in a real editor, because nothing makes the editor fire a real copy (`editors/vscode/src/imports.ts`, § *Known gaps*).
- A cursor on an attribute's name, as in `#[Core\Ro<|>ute(...)]`, is offered every `Core` type rather than the attributes the site allows. No `.lspt` case freezes that answer yet; the payload case is `tests/lsp/completion/an-enum-name-inside-an-attribute-payload-offers-its-cases.lspt`.
- `semanticTokenScopes` needs no new entry: every token type is in LSP's standard legend, and only the two modifiers are mapped (`docs/rules/ide/novis-ships-names-not-colours.md`).
