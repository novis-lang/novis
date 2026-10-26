---
looper: 1
title: The editor reads what the program and its recorded requests did
summary: "A method's lens counts the routes that reach it and the tests that ran it and runs those tests, call hierarchy walks the same call graph, a route's hover shows the program's grants, a tainted or secret value's hover shows its path, each query call shows what a recorded request ran there, a recorded request becomes a test that asserts its response, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/hover.rs]
stages:
  - id: 1
    title: the call graph, and the routes and tests that reach a method
    status: done
    context: [crates/nvs-lsp/src/index.rs, crates/nvs-types/src/routes.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-stdlib/src/test.rs, docs/rules/ide/used-by-counts-the-routes-and-tests-that-reach-a-method.md, docs/rules/ide/five-features-are-one-reference-index.md]
    extra:
      context:
        rules: [ide/used-by-counts-the-routes-and-tests-that-reach-a-method, ide/five-features-are-one-reference-index, testing/in-process-request]
  - id: 8
    title: a "used by" lens reads the coverage report and runs its tests
    status: open
    context: [crates/nvs-lsp/src/server.rs, editors/vscode/src/tests.ts, editors/vscode/package.json, docs/rules/ide/used-by-counts-the-routes-and-tests-that-reach-a-method.md, docs/rules/ide/the-request-set-is-closed.md, docs/rules/testing/report-formats.md]
    extra:
      context:
        rules: [ide/used-by-counts-the-routes-and-tests-that-reach-a-method, ide/the-request-set-is-closed, ide/the-test-tree-is-discovered-and-run-through-the-cli, ide/contributions-are-frozen-and-only-ever-added, ide/an-lsp-answer-is-frozen-as-an-lspt-case, testing/report-formats, testing/coverage-report]
  - id: 2
    title: call hierarchy walks the call graph both ways
    status: open
    context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/capabilities.rs, crates/nvs-lsp/tests/handshake.rs, docs/rules/ide/the-request-set-is-closed.md, docs/rules/ide/five-features-are-one-reference-index.md]
    extra:
      context:
        rules: [ide/the-request-set-is-closed, ide/five-features-are-one-reference-index, ide/used-by-counts-the-routes-and-tests-that-reach-a-method, ide/an-lsp-answer-is-frozen-as-an-lspt-case]
  - id: 3
    title: a route's hover shows the program's grants
    status: open
    context: [crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/document.rs, crates/nvs-config/src/snapshot.rs, crates/nvs-config/src/tree.rs, docs/rules/ide/a-route-hover-shows-the-programs-grants.md]
    extra:
      context:
        rules: [ide/a-route-hover-shows-the-programs-grants, security/capability-question-is-grant-and-scope, ide/the-request-set-is-closed, ide/an-lsp-answer-is-frozen-as-an-lspt-case]
  - id: 4
    title: a `tainted` or `secret` value's hover shows its path
    status: open
    context: [crates/nvs-types/src/expr/quals.rs, crates/nvs-types/src/expr_table.rs, crates/nvs-types/src/check.rs, crates/nvs-lsp/src/document.rs, crates/nvs-lsp/src/hover.rs, docs/rules/ide/a-hover-shows-where-a-tainted-or-secret-value-came-from.md]
    extra:
      context:
        rules: [ide/a-hover-shows-where-a-tainted-or-secret-value-came-from, security/tainted-sources, security/launderers-are-sink-named, security/taint-propagation, ide/tainted-has-no-default-decoration]
  - id: 5
    title: a query call passes its site id, and a traced span records the site
    status: open
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/lower/anon_fn.rs, crates/nvs-ir/src/lower/call.rs, crates/nvs-stdlib/src/db/span.rs, crates/nvs-stdlib/src/db/execute.rs, crates/nvs-db/src/span.rs, docs/rules/observability/a-query-span-names-its-call-site.md]
    extra:
      context:
        rules: [observability/a-query-span-names-its-call-site, observability/a-query-is-a-trace-event, core-classes/db-compile-time-query-checking, security/reflection-enforces-visibility]
  - id: 6
    title: each query call shows what the newest recorded request ran there
    status: open
    context: [crates/nvs-lsp/src/server.rs, crates/nvs-config/src/data.rs, viewer/src/views/compare.tsx, docs/rules/ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site.md]
    extra:
      context:
        rules: [ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site, observability/a-query-span-names-its-call-site, testing/the-ingester-runs-whether-or-not-anyone-is-looking, testing/debug-surface]
  - id: 7
    title: a recorded request becomes a test that asserts its response
    status: open
    context: [crates/nvs-cli/src/debug.rs, crates/nvs-stdlib/src/test.rs, crates/nvs-fmt/src/lib.rs, viewer/src/views/request.tsx, docs/rules/testing/a-recorded-request-becomes-a-test.md]
    extra:
      context:
        rules: [testing/a-recorded-request-becomes-a-test, testing/in-process-request, testing/a-recorded-request-keeps-its-input-with-credentials-replaced, testing/feature-proofs]
  - id: 9
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: owners-closes
    stage: 7
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, request-aware-editor]
    reach: true
    floor: false
---

# The editor reads what the program and its recorded requests did

## Why

The editor answers questions about a file: a type, a definition, a reference count. It cannot answer
the questions a developer asks about a running web application. Which routes and tests reach this
method? What may this route's program touch? Where did this `tainted` value come from? How many
queries did that request run on this line? A code lens has the reference count and nothing else
(`crates/nvs-lsp/src/server.rs:1173`). No reference records the function it is written in
(`crates/nvs-lsp/src/index.rs:277`), so no reader can walk from a method to its callers. A query span
does not know which line sent it (`crates/nvs-db/src/span.rs:87`). The checker keeps no record
of where a qualifier came from. This goal builds those four readings on the hover, the code lens and
call hierarchy, and turns a request `debug-service` recorded into a test.

## Standing decisions

**A query knows its call site through a constant site id, always.** The user decided this. The
compiler passes a compile-time site id, one integer constant, to each `Core\Db` call that sends a
statement, in every mode. The span stores it only when tracing is on.
`rule:observability/a-query-span-names-its-call-site` holds it.

**"Used by N routes" is transitive.** The user decided this. A route counts when its handler reaches
the method through a chain of calls the checker resolved. Calls through a `callable` or a `mixed`
receiver are not followed, and the lens says so. The chain is a call graph read off the reference
index: each occurrence records the function it is written in.
`rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method` holds it.

**"M tests" comes from per-test coverage, and the call graph where there is none.** The user decided
this. The count is the tests the per-test coverage of the last `nvs test` run with coverage saw reach
the method, which is exact (`rule:testing/report-formats`). Where no such report exists, the lens
counts tests by the call graph and marks the count "static". A click runs exactly the tests counted.
The same rule holds it.

**Call hierarchy is offered.** The user decided this. Once the call graph exists,
`textDocument/callHierarchy` answers incoming and outgoing calls from it. A call through a `callable`
or a `mixed` receiver is not shown. `rule:ide/the-request-set-is-closed` admits it, and stage 2 builds it.

**The route-reach hover shows the program's grants**, labelled as that program's, with no per-route
narrowing. The user decided this. `rule:ide/a-route-hover-shows-the-programs-grants` holds it.

**The taint path is recorded only where `nvs-lsp` asks.** The planning session's call, as the plan
already said. A switch only `nvs-lsp` sets makes the checker record a qualified value's provenance, so a
compile for running pays nothing. `rule:ide/a-hover-shows-where-a-tainted-or-secret-value-came-from`.

**The query lens reads the newest recorded request that reached the site, and keeps no parameter.**
The planning session's call. No query span holds a bound parameter
(`rule:observability/a-query-is-a-trace-event`), so the plan's "one parameter set per site" is not
built: the lens shows the statement as written. A record made from an older text of the file is not
shown. `rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`.

**A recorded request becomes a test that asserts its response.** The user decided this. `nvs debug
to-test` prints a test that sends the recorded request through `Core\Test::request` and asserts the
status, the key headers and the body, a JSON body compared structurally. The spool records the
response body up to `[limits] request_body`
(`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`), and a body cut at that
cap gives a test that asserts the status and the headers only. A credential the spool redacted is
written as a placeholder with a comment. This goal also adds the viewer's button that calls the same
function, because `debug-viewer` leaves it to the generator's goal.
`rule:testing/a-recorded-request-becomes-a-test`.

**The feature id for `nvs debug to-test` is `tools:cli/nvs-debug-to-test`.** It is a command a
developer types, so it gets its own `# nvs debug to-test` section in `docs/reference/tools/10-cli.md`,
after the `# nvs debug` section `debug-service` adds, with its help, examples and proofs. The editor
readings are `tools:editor/nvs-lsp`'s: their `.lspt` cases carry that feature's `covers:` marker and
`docs/reference/tools/40-editor.md` gains a paragraph each. Why: one feature per command keeps each
proof set small and findable, and the readings add to the language server rather than standing alone.

**One new standard request, and no new request of Novis's own.** Call hierarchy is the user's
decision. Every other feature here is a `codeLens` or a `hover`
(`rule:ide/the-extension-builds-no-ui-the-editor-already-has`), so
`rule:ide/the-request-set-is-closed` gains one standard request and a sentence per new reading.

**Gates.** Stage 8 needs a lens that carries a command: stage 4 of `docs/goals/editor-fixes.md` is
`done`, or that file is gone. Stage 6 needs the debug index and the compare view: stage 4 of
`docs/goals/debug-service.md` and stage 6 of `docs/goals/debug-viewer.md` are `done`, or those files
are gone. Stage 7 needs the recorded request and response and the request view: `docs/goals/debug-service.md`
is gone (or its stages 3, 4 and 7 are `done`) and stage 8 of `docs/goals/debug-viewer.md` is `done`.
Stages 1, 2, 3, 4 and 5 need no other goal. A session whose gate fails takes the next stage of the lanes
below that has none, and says so in the handoff.

**Lanes.** Stage 5 (`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-ir/src/lower/`,
`crates/nvs-stdlib/src/db/`, `crates/nvs-db/src/span.rs`) is disjoint from stages 1 to 4 and can run
now as a side goal. Stages 1, 2 and 8 share `crates/nvs-lsp/src/index.rs` and
`crates/nvs-lsp/src/server.rs` and run in order 1, 2, 8 (8 only once its gate holds). Stage 8's client
half (`editors/vscode/src/tests.ts`) is its own lane once the lens command's arguments are fixed. Stages 3
and 4 share `crates/nvs-lsp/src/hover.rs` and `crates/nvs-lsp/src/document.rs` and run in order; they
touch nothing of stages 1 and 2 but `server.rs`'s hover entry, so a side goal may run them. Stage 6
follows 5, and 7 runs last.

**Narrow test commands.** The `.lspt` cases of one directory run under `nvs lsp-test tests/lsp/lens/`
or `nvs lsp-test tests/lsp/hover/` (`tests/lsp/README.md:8`); the handshake list under `cargo test --test handshake`; the extension's client test under
`npm run test:headless` in `editors/vscode`; never the VS Code host suite. The group's one verify is
`bun nv affected --run`.

**Trade-offs, all together.** Request path: one constant argument per query call in every mode, and
one more integer in a span only when it is traced. Compile path: nothing outside `nvs-lsp`, where the
taint path costs a step list per qualified value. Memory in the editor: one function name per
occurrence in the index, the last coverage report's per-test lines while the server holds them, and a
few numbers per query site per lens. Usability: four answers no other PHP-family tool gives from the
compiler's own tables, call hierarchy, and a test written from a real request. Simplicity: one new
standard request, one new client command, `nvs.runTests`, and one new subcommand, `nvs debug to-test`.

## Stages

### 1. the call graph, and the routes and tests that reach a method

**Does:** each reference knows its enclosing function, and a walk from a method finds the routes and
the `#[Test]` methods that reach it.

- [x] Each `Occurrence` records the function or method it is written in, as the index builds it (`rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method`). `crates/nvs-lsp/src/index.rs:277`, `crates/nvs-lsp/src/index.rs:1200`
- [x] Reach walks callers backwards from a method, each function once, to the route rows' handlers and the `#[Test]` methods, and matches a literal `Core\Test::request` path with the server's matcher. `crates/nvs-types/src/routes.rs:324`, `crates/nvs-types/src/expr_table.rs:1460`, `crates/nvs-runtime/src/routes.rs:846`, `crates/nvs-stdlib/src/test.rs:743`
- [x] Rust tests of the walk: a direct, a transitive, a recursive and an "at least" reach, and a static test count. `crates/nvs-lsp/tests/index.rs:1`
- [x] `rule:ide/five-features-are-one-reference-index` rewritten where it says no call-site edges exist. `docs/rules/ide/five-features-are-one-reference-index.md:18`

### 8. a "used by" lens reads the coverage report and runs its tests

**Does:** a method's lens counts the routes that reach it and the tests that ran it, and runs those
tests.

**Split:** after the session lands the lens and its command, a writing subagent writes the `.lspt`
cases under `tests/lsp/lens/` as one batch; the client half in `editors/vscode/` may go to a second
writing subagent after that.

- [ ] The lens above a method says "used by N routes, M tests", with "at least" where a `callable` or a `mixed` call may reach it. Its command carries the tests. `crates/nvs-lsp/src/server.rs:1173`, `crates/nvs-lsp/src/server.rs:1200`
- [ ] The Coverage run profile also keeps the run's JSON report with its per-test lines, and the server reads the tests that reached a method from the newest one. With no report, the test count comes from the call graph and the lens marks it "static" (`rule:testing/report-formats`). `editors/vscode/src/tests.ts:92`, `editors/vscode/src/tests.ts:115`
- [ ] The client's `nvs.runTests` command runs exactly those tests through the Test Explorer's filtered run (`rule:ide/contributions-are-frozen-and-only-ever-added`). `editors/vscode/src/tests.ts:218`, `editors/vscode/package.json:202`
- [ ] `.lspt` cases for a direct, a transitive, a recursive and an "at least" count, a test count read from a report and a "static" one (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`). `tests/lsp/lens/a-method-counts-the-routes-that-reach-it.lspt:1`
- [ ] A client test for the click. `editors/vscode/scripts/headless.mjs:1`
- [ ] `rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method` `shipped`. `data/rules/ide/used-by-counts-the-routes-and-tests-that-reach-a-method.json:3`

### 2. call hierarchy walks the call graph both ways

**Does:** `textDocument/callHierarchy` answers incoming and outgoing calls from the call graph stage 1
built.

- [ ] The server declares `callHierarchyProvider`, and `crates/nvs-lsp/tests/handshake.rs` holds the eighteenth standard request (`rule:ide/the-request-set-is-closed`). `crates/nvs-lsp/src/capabilities.rs:162`, `crates/nvs-lsp/tests/handshake.rs:161`
- [ ] `prepare` resolves the function or method under the cursor, `incomingCalls` lists the functions whose occurrences resolved to it, and `outgoingCalls` lists the functions it calls, each with the call ranges. `crates/nvs-lsp/src/index.rs:277`, `crates/nvs-lsp/src/server.rs:1173`
- [ ] A call through a `callable` value or a `mixed` receiver is not shown, as in the lens. `crates/nvs-lsp/src/index.rs:277`
- [ ] A Rust test for both directions over a recursive pair and a call through a `callable`. `crates/nvs-lsp/tests/navigation.rs:1`
- [ ] `rule:ide/the-request-set-is-closed` rewritten from "admitted" to served. `docs/rules/ide/the-request-set-is-closed.md:72`

### 3. a route's hover shows the program's grants

**Does:** hovering a `#[Route]` or its handler's name lists what the program's grants allow, and names
the `nvs.toml` it read.

**Split:** after the session lands the hover, a writing subagent writes the `.lspt` cases and their
fixture under `tests/lsp/hover/` as one batch.

- [ ] The hover resolves the snapshot for the program's entry file the way the extension set does, once per hover (`rule:ide/a-route-hover-shows-the-programs-grants`). `crates/nvs-lsp/src/document.rs:809`, `crates/nvs-config/src/snapshot.rs:149`
- [ ] It lists the grants by kind with their scopes, under a first line naming the file and saying the grants are the program's (`rule:security/capability-question-is-grant-and-scope`). `crates/nvs-config/src/tree.rs:501`, `crates/nvs-lsp/src/hover.rs:132`
- [ ] With no `nvs.toml`, it says so and names the default policy. `crates/nvs-lsp/src/hover.rs:132`
- [ ] `.lspt` cases over a fixture with grants, a changed grant giving a changed hover with no source edit, and none. `tests/lsp/hover/a-route-hover-lists-the-programs-grants.lspt:1`
- [ ] `rule:ide/a-route-hover-shows-the-programs-grants` `shipped`. `data/rules/ide/a-route-hover-shows-the-programs-grants.json:3`

### 4. a `tainted` or `secret` value's hover shows its path

**Does:** under a switch only `nvs-lsp` sets, the checker records where a qualifier entered a method and
each step it took, and the hover shows it.

**Split:** after the session lands the switch and the recorded steps, a writing subagent writes the
`.lspt` cases under `tests/lsp/hover/` as one batch.

- [ ] A switch on the expression table that only `nvs-lsp` sets, and a test that `nvs check` and a run leave it off (`rule:ide/a-hover-shows-where-a-tainted-or-secret-value-came-from`). `crates/nvs-types/src/expr_table.rs:1428`, `crates/nvs-types/src/check.rs:128`, `crates/nvs-lsp/src/document.rs:725`
- [ ] Under it, the checker records the source, each assignment and concatenation that carried the qualifier, and the launderer (`rule:security/taint-propagation`, `rule:security/launderers-are-sink-named`). `crates/nvs-types/src/expr/quals.rs:414`, `crates/nvs-types/src/expr/quals.rs:523`, `crates/nvs-types/src/expr/quals.rs:574`
- [ ] Hover on the value, or on an argument at a sink, shows the steps with their lines and the launderer marked. `crates/nvs-lsp/src/hover.rs:132`
- [ ] `.lspt` cases for a `tainted` and a `secret` value from source to sink, one laundered on the way, and one that entered through a declared parameter. `tests/lsp/hover/a-tainted-value-shows-where-it-came-from.lspt:1`
- [ ] `rule:ide/a-hover-shows-where-a-tainted-or-secret-value-came-from` `shipped`. `data/rules/ide/a-hover-shows-where-a-tainted-or-secret-value-came-from.json:3`

### 5. a query call passes its site id, and a traced span records the site

**Does:** each `Core\Db` call that sends a statement gets a constant site id, and a traced span carries
the file and line it names.

- [ ] A roster of the `Core\Db` members that send a statement, on the pattern `CALL_SITE_MEMBERS` and `PATH_GRAPH_MEMBERS` use, and the rule fixes the argument's slot (`rule:observability/a-query-span-names-its-call-site`). `crates/nvs-stdlib/src/registry.rs:3055`, `crates/nvs-stdlib/src/registry.rs:3139`
- [ ] The lowering passes the id at each call, where it hands a `CALL_SITE_MEMBERS` member its constant, and the program keeps a site table: file, line and the file's text hash. `crates/nvs-ir/src/lower/expr.rs:4201`, `crates/nvs-ir/src/lower/mod.rs:2541`, `crates/nvs-ir/src/lower/call.rs:83`
- [ ] The callable-reference thunk passes zero. `crates/nvs-ir/src/lower/anon_fn.rs:818`
- [ ] The span records the site only when it is recorded. `crates/nvs-stdlib/src/db/span.rs:52`, `crates/nvs-stdlib/src/db/execute.rs:1051`, `crates/nvs-db/src/span.rs:87`
- [ ] Tests: two calls with the same SQL are two sites, an untraced call reads no site, and `a_query_span_contains_no_parameter_value_anywhere` still passes. `crates/nvs-db/src/pg.rs:6627`
- [ ] `rule:observability/a-query-span-names-its-call-site` `shipped`. `data/rules/observability/a-query-span-names-its-call-site.json:3`

### 6. each query call shows what the newest recorded request ran there

**Does:** with `[debug] store` on, a lens above each query call reads the debug index for the newest
recorded request that reached the site.

- [ ] The server opens the debug index read-only from the data folder, and an index that is absent or locked gives no lens (`rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`). `crates/nvs-config/src/data.rs:119`
- [ ] The lens shows the count, the total and the slowest time at the site, and "repeated" with the count when one statement ran there more than once. Its click opens the request in the viewer. `crates/nvs-lsp/src/server.rs:1173`
- [ ] A record whose file hash is not the file on disk is not shown. `crates/nvs-lsp/src/server.rs:1173`
- [ ] The viewer's compare aligns two requests' queries by site, which `debug-viewer` leaves to this goal. `viewer/src/views/compare.tsx:1`
- [ ] A served fixture that runs one query in a loop of many iterations shows one lens at that site with the right count, flagged as repeated, and its record does not grow with the iteration count. `tests/lsp/lens/a-query-run-in-a-loop-is-one-repeated-lens.lspt:1`
- [ ] `rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site` `shipped`. `data/rules/ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site.json:3`

### 7. a recorded request becomes a test that asserts its response

**Does:** `nvs debug to-test <trace-id>` prints a `Core\Test::request` case for a recorded request that
asserts the recorded response.

**Split:** after the command lands, a writing subagent writes the feature's `about.md` and three
examples under `docs/examples/tools/cli/nvs-debug-to-test/`; the session writes the rule flip.

- [ ] The command reads the request and its response from the debug index and prints one class with one `#[Test]` method, through the formatter's printer (`rule:testing/a-recorded-request-becomes-a-test`). `crates/nvs-cli/src/debug/to_test.rs:1`, `crates/nvs-stdlib/src/test.rs:743`, `crates/nvs-fmt/src/lib.rs:134`
- [ ] The method asserts the status, `Content-Type`, `Location` on a redirect, and the body: a JSON body through `json()` compared structurally, any other body as bytes. A body the spool cut at `request_body` gives a test that asserts the status and the headers only. `crates/nvs-cli/src/debug/to_test.rs:1`
- [ ] A header the record holds as `[redacted]` is written as the placeholder with its comment, and a request whose recorded request body is not the whole body is refused with the reason (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`). `crates/nvs-cli/src/debug/to_test.rs:1`
- [ ] `--write <file>` writes a new file and never replaces one. `crates/nvs-cli/src/debug/to_test.rs:1`
- [ ] The viewer's request view gains the button that shows the printed test with a copy button. `viewer/src/views/request.tsx:1`
- [ ] A served fixture: the generated test passes against the unchanged program, and fails when its handler's status, its `Content-Type` or one JSON field of its body changes. `crates/nvs-cli/tests/debug_to_test.rs:1`
- [ ] The help text and a `# nvs debug to-test` section, so `bun nv proofs --id 'tools:cli/nvs-debug-to-test'` prints nothing owed (`rule:testing/feature-proofs`). `docs/reference/tools/10-cli.md:302`, `docs/examples/tools/cli/nvs-debug-to-test/about.md:1`
- [ ] `rule:testing/a-recorded-request-becomes-a-test` `shipped`, and `bun nv owners --closes request-aware-editor` passes. `data/rules/testing/a-recorded-request-becomes-a-test.json:3`

### 9. the goal is verified once, and every failure is fixed

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

Stage 1 is done. Each `Occurrence` carries `caller`, the method whose body it is written in, and
`None` for a callable value and for code outside a method. Each indexed file also keeps `requests`
(a `Core\Test::request` with a literal method and path, matched to its handler by
`nvs_types::crossing::runtime_routes` and the server's `match_request`) and `unfollowed` (a method
taken as a callable value, and a by-name call that resolved to nothing, as `::name`). A method's
`Declaration` carries `routes` (`GET /items/{id}`) and `test`. `nvs_lsp::reach::reach` walks the
edges backwards and answers `Reach { routes, tests, at_least }`. A call to a method the reached
method overrides is not followed and sets `at_least`, and a closure's calls count for the method that
writes it; both are written into `rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method`.
`tests/index.rs` counts the call graph as the eighth reader, and its five new tests pass.

The route-table crossing moved from `nvs-cli` into `crates/nvs-types/src/crossing.rs`, and `nvs-lsp`
names `nvs-runtime`. The gates of stages 8, 6 and 7 hold: `editor-fixes`, `debug-service` and
`debug-viewer` are all gone.

No test is expected to go red. `call_hierarchy_is_not_answered` in `crates/nvs-lsp/tests/index.rs`
must go in stage 2, which declares the capability.

### Next

**Stage 2, call hierarchy**, on `crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/capabilities.rs`
and a new module beside `crates/nvs-lsp/src/reach.rs`. Gate: none.

- [ ] Declare `callHierarchyProvider`, and the handshake list holds the new standard request. Delete `call_hierarchy_is_not_answered` and the playbook bullet whose `until:` names it. `crates/nvs-lsp/src/capabilities.rs:162`, `crates/nvs-lsp/tests/handshake.rs:161`, `crates/nvs-lsp/tests/index.rs:300`
- [ ] `prepare` reads the method under the cursor with `index::symbol_at`. `incomingCalls` groups `index.occurrences(m)` by `caller`, and adds `index.requests(m)` when that is the design chosen. `outgoingCalls` reads `index.occurrences_in(file)` where `caller == m` and the symbol is a method. `crates/nvs-lsp/src/index.rs:836`, `crates/nvs-lsp/src/reach.rs:1`
- [ ] A callable value has `caller: None`, so it is never shown. A Rust test covers both directions over a recursive pair and a callable. `crates/nvs-lsp/tests/navigation.rs:1`
- [ ] `rule:ide/the-request-set-is-closed` rewritten from "admitted" to served. `docs/rules/ide/the-request-set-is-closed.md:72`

Stage 8 comes after it. Its lens reads `reach::reach`.

### Backlog

- Stage 5 is disjoint and can run as a side goal now (`docs/rules/observability/a-query-span-names-its-call-site.md`).
- A test count read from a coverage report is as old as that run; the next coverage run refreshes it (`docs/rules/ide/used-by-counts-the-routes-and-tests-that-reach-a-method.md`).
- `nvs dap` answers no `evaluate` for watch and hover expressions, and no rule asks for it. A goal that wants it writes it into the rule first (`docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md`).
- A breakpoint condition that calls a pure `Core` function needs a purity mark on `Core` methods (`docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md`).
- An `inout` by-reference variable has no debug slot, so a stopped frame does not list it. The adapter can read it through its staged cell (`crates/nvs-ir/src/lower/mod.rs:2602`).
- No test checks the name the call stack gives an anonymous function's frame (`crates/nvs-cli/tests/debugger.rs:241`).
- `nvs dap` sends `exited` once every holder of the program's pipes closes them, so a child process the program left running delays it (`crates/nvs-cli/src/dap/mod.rs:223`).
