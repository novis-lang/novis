---
milestone: M10
---
# Loop goal 39 — The extension answers everything it contributes

Goal `editor` shipped an extension whose manifest promises more than its code answers, and goal `workspace-index` gives
that code a server worth wiring to. This goal closes the gap on the client side: the three commands
that have been contributed and unanswered since M4B, the Tasks a landed rule already requires, a
native Test Explorer, the AST panel, and HTML/CSS/JS services inside an inline-HTML region. The two
CLI surfaces those need — `nvs check --json` and test discovery — land here with them, because a
client feature whose data does not exist is not a client feature.

Nothing here needs `nvs fmt`, `nvs dap` or the profiler, so nothing here waits on them. What they
gate — format-on-save, the debugger UI, the profile view, and coverage in the Test Explorer — stays
out, and `rule:ide/every-feature-is-staged-behind-its-dependency` is why.

Goal `workspace-index`'s whole acceptance list is this goal's floor, and it is never traded.

## Why here

The editor half, and the two CLI surfaces it queries. Split from goal `workspace-index` rather than run with it
because the two share no file set — one is a Rust crate and a protocol, the other is TypeScript over
a command line — and a stalled index should not hold up a `problemMatcher` that is two lines of
regex over a rendering that has existed since M0. It also closes goal `editor`'s three
contributed-and-unanswered command ids, which
`rule:ide/contributions-are-frozen-and-only-ever-added` will not let anyone delete.

## Stage 0 — the catch-up

**Three commands are contributed and answer nothing.** `nvs.run`, `nvs.test` and `nvs.showAst` are
in the frozen roster at `editors/vscode/package.json` and
`rule:ide/contributions-are-frozen-and-only-ever-added` keeps them there, but
`editors/vscode/src/extension.ts` registers only `nvs.restartServer`, `nvs.revealSecret` and
`nvs.hideSecrets`. A user who runs one from the palette gets *command not found*. Stages 3 and 4
answer them; this stage is the failing test that says they are unanswered, and the playbook bullet
at `docs/agent/playbook.md:1261` is deleted when it passes.

**And one setting is inert.** `nvs.lsp.debounce` is declared in the manifest and read by nobody —
the client passes no `initializationOptions`, so the server never sees it. Either it reaches the
server or it stops being contributed; the roster's rule says a name is never removed, so it reaches
the server.

## Stage 1 — the floor

Goal `workspace-index`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for anything above it.

## Stage 2 — the keystone: the two CLI surfaces

Both are data the compiler already has, rendered a second way. They go first because two client
features are queries against them.

1. **`nvs check --json`**, per `rule:ide/check-json-is-the-diagnostic-record-as-a-document`: one
   record per diagnostic the text renderer prints — code, span, severity, help and `suggestions` —
   under a schema frozen the way `nvs ast --json`'s is. The text rendering stays the default. This
   is a second renderer over `nvs-diagnostics`' existing `Diagnostic`, not a second pipeline.
2. **Test discovery in `nvs test`.** `--format json` today emits `class`, `method`, `verdict`,
   `durationMs` and `failures` and no location, and there is no way to enumerate tests without
   running them — so a Test Explorer cannot place a test in a file or populate itself before a run.
   This stage adds the declaring file and line to each record, and a listing mode that discovers
   without executing. `schemaVersion` goes to 2 and the JUnit and human formats are untouched.

## Stage 3 — the Tasks, and the Problems panel

`rule:ide/tasks-carry-a-problem-matcher` is landed and unimplemented: `contributes` holds no
`taskDefinitions` and no `problemMatchers` at all today.

1. **`nvs run` and `nvs test` as contributed Tasks**, each carrying a `problemMatcher` — a two-line
   regex over the renderer's existing format, `error[E0301]: message` then `  --> file:line:col`
   (`rule:errors/renderings`). Without it the Tasks print into a terminal; with it every diagnostic
   is a clickable Problems entry.
2. **`nvs.run` and `nvs.test` answered**, as the commands that start those Tasks rather than as a
   second way to spawn a process. One execution path, two entry points.

## Stage 4 — the AST panel

`nvs.showAst` answered, per `rule:ide/the-ast-panel-shells-out-to-the-cli`: the panel runs
`nvs ast --json` and renders what comes back. No parser in the client, and no second schema — 
`rule:ide/ast-json-schema-is-frozen` is the one the CLI already emits.

## Stage 5 — the Test Explorer

VS Code's native `TestController`, wired to stage 2's discovery and run surfaces.

1. **Discovery** from the listing mode, so the tree is populated before anything runs.
2. **Running** a test, a class or the tree through `nvs test --format json`, with each record's
   verdict and failures becoming the test item's state and message.
3. **`.nvst` cases** run through the same controller as a second suite, since
   `rule:ide/lspt-coverage-is-inferred`'s corpus and `nvs test`'s methods are two rosters and one
   explorer.
4. **No coverage.** VS Code's `FileCoverage` API is wired to the Clover/lcov exporters, and those do
   not exist. The Explorer ships without it rather than with a gutter of its own.

## Stage 6 — the inline-HTML region

`nvs/regions`, the second request of Novis's own, already named in
`rule:ide/contributions-are-frozen-and-only-ever-added`'s M10 roster.

1. **The server reports the regions.** `TokenKind::InlineHtml` is already what the lexer emits and
   `StmtKind::InlineHtml` what the parser holds, so the request is a projection of the tree rather
   than a feature built on it.
2. **The client forwards** to VS Code's own HTML, CSS and JavaScript services across those
   boundaries: Emmet, tag closing and renaming, the colour picker, validation. Behind
   `nvs.template.services`.
3. **And no formatting.** A second formatter inside a `.nvs` file is exactly what
   `rule:tooling/fmt-is-never-a-diagnostic` exists to prevent, and `nvs fmt` does not exist to be
   the first one.

## Standing decisions

- **A contributed identifier is answered or it is a failing test.** The roster is frozen by
  `rule:ide/contributions-are-frozen-and-only-ever-added`, so an unanswered command cannot be
  removed — it can only be answered. Stage 0's test is what stops the next one being added and
  forgotten for a milestone.
- **The Test Explorer ships without coverage, and this is not a partial feature.** Coverage is a
  different data source that does not exist. Wiring `FileCoverage` to nothing, or drawing a gutter
  in the client, are both refused — the first is a lie and the second is the custom UI
  `rule:ide/the-extension-builds-no-ui-the-editor-already-has` forbids.
- **`schemaVersion` goes to 2 rather than growing a parallel document.** The test JSON is consumed
  by CI as well as by this explorer, and one schema with a version is what
  `rule:ide/ast-json-schema-is-frozen` already established as the shape for this.
- **No formatting, no debugging, no profiling, in any form.** Not stubbed, not flagged, not
  returning empty. Each waits for the binary that answers it.
- **This goal opens one ADR**, covering stage 2's two schemas and stage 5's explorer shape. Stages
  3, 4 and 6 implement rules that are already landed and need no new record.
- **Where ambiguity resolves:** a client feature whose CLI surface is missing is stage 2's problem,
  not a client-side reconstruction. Nothing here parses a human-format report to recover a field
  the JSON should have carried.
