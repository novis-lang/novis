---
looper: 1
title: The editor reads what the program and its recorded requests did
summary: "A method's lens counts the routes that reach it and the tests that ran it and runs those tests, call hierarchy walks the same call graph, a route's hover shows the program's grants, a tainted or secret value's hover shows its path, each query call shows what a recorded request ran there, a recorded request becomes a test that asserts its response, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/hover.rs]
stages:
  - id: 1
    title: the call graph, and a "used by" lens that runs its tests
    status: open
    context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/server.rs, crates/nvs-types/src/routes.rs, crates/nvs-runtime/src/routes.rs, editors/vscode/src/tests.ts, editors/vscode/package.json, docs/rules/ide/used-by-counts-the-routes-and-tests-that-reach-a-method.md, docs/rules/ide/five-features-are-one-reference-index.md, docs/rules/ide/the-request-set-is-closed.md, docs/rules/testing/report-formats.md]
    extra:
      context:
        rules: [ide/used-by-counts-the-routes-and-tests-that-reach-a-method, ide/five-features-are-one-reference-index, ide/the-request-set-is-closed, ide/the-test-tree-is-discovered-and-run-through-the-cli, ide/contributions-are-frozen-and-only-ever-added, testing/in-process-request, testing/report-formats, testing/coverage-report]
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
        rules: [ide/a-route-hover-shows-the-programs-grants, security/capability-question-is-grant-and-scope, ide/the-request-set-is-closed]
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
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-ir/src/lower/anon_fn.rs, crates/nvs-ir/src/lower/call.rs, crates/nvs-stdlib/src/db/span.rs, crates/nvs-stdlib/src/db/execute.rs, crates/nvs-db/src/span.rs, docs/rules/observability/a-query-span-names-its-call-site.md]
    extra:
      context:
        rules: [observability/a-query-span-names-its-call-site, observability/a-query-is-a-trace-event, core-classes/db-compile-time-query-checking, security/reflection-enforces-visibility]
  - id: 6
    title: each query call shows what the newest recorded request ran there
    status: open
    context: [crates/nvs-lsp/src/server.rs, crates/nvs-config/src/data.rs, docs/rules/ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site.md]
    extra:
      context:
        rules: [ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site, observability/a-query-span-names-its-call-site, testing/the-ingester-runs-whether-or-not-anyone-is-looking, testing/debug-surface]
  - id: 7
    title: a recorded request becomes a test that asserts its response
    status: open
    context: [crates/nvs-stdlib/src/test.rs, crates/nvs-fmt/src/lib.rs, docs/rules/testing/a-recorded-request-becomes-a-test.md]
    extra:
      context:
        rules: [testing/a-recorded-request-becomes-a-test, testing/in-process-request, testing/a-recorded-request-keeps-its-input-with-credentials-replaced]
checks:
  - id: owners-closes
    stage: 7
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, request-aware-editor]
    reach: true
    floor: false
extra:
  milestone: M10
---

# The editor reads what the program and its recorded requests did

## Why

The editor answers questions about a file: a type, a definition, a reference count. It cannot answer
the questions a developer asks about a running web application. Which routes and tests reach this
method? What may this route's program touch? Where did this `tainted` value come from? How many
queries did that request run on this line? A code lens has the reference count and nothing else
(`crates/nvs-lsp/src/server.rs:1198`). No reference records the function it is written in
(`crates/nvs-lsp/src/index.rs:271`), so no reader can walk from a method to its callers. A query span
does not know which line sent it (`crates/nvs-db/src/span.rs:91-113`). The checker keeps no record
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

**One new standard request, and no new request of Novis's own.** Call hierarchy is the user's
decision. Every other feature here is a `codeLens` or a `hover`
(`rule:ide/the-extension-builds-no-ui-the-editor-already-has`), so
`rule:ide/the-request-set-is-closed` gains one standard request and a sentence per new reading.

**Trade-offs, all together.** Request path: one constant argument per query call in every mode, and
one more integer in a span only when it is traced. Compile path: nothing outside `nvs-lsp`, where the
taint path costs a step list per qualified value. Memory in the editor: one function name per
occurrence in the index, the last coverage report's per-test lines while the server holds them, and a
few numbers per query site per lens. Usability: four answers no other PHP-family tool gives from the
compiler's own tables, call hierarchy, and a test written from a real request. Simplicity: one new
standard request, one new client command, `nvs.runTests`, and one new subcommand, `nvs debug to-test`.

## Stages

### 1. the call graph, and a "used by" lens that runs its tests

**Does:** each reference knows its enclosing function, and a method's lens counts the routes that reach
it and the tests that ran it, and runs those tests.

- [ ] Each `Occurrence` records the function or method it is written in, as the index builds it (`rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method`). `crates/nvs-lsp/src/index.rs:271`, `crates/nvs-lsp/src/index.rs:1183`
- [ ] Reach walks callers backwards from a method, each function once, to the route rows' handlers and the `#[Test]` methods, and matches a literal `Core\Test::request` path with the server's matcher. `crates/nvs-types/src/routes.rs:339`, `crates/nvs-types/src/expr_table.rs:2058`, `crates/nvs-runtime/src/routes.rs:831`, `crates/nvs-stdlib/src/test.rs:739`
- [ ] The Coverage run profile also keeps the run's JSON report with its per-test lines, and the server reads the tests that reached a method from the newest one. With no report, the test count comes from the call graph and the lens marks it "static". `editors/vscode/src/tests.ts:92`, `editors/vscode/src/tests.ts:115`
- [ ] The lens above a method says "used by N routes, M tests", with "at least" where a `callable` or a `mixed` call may reach it. Its command carries the tests. `crates/nvs-lsp/src/server.rs:1173`, `crates/nvs-lsp/src/server.rs:1450`
- [ ] The client's `nvs.runTests` command runs exactly those tests through the Test Explorer's filtered run. `editors/vscode/src/tests.ts:218-226`, `editors/vscode/package.json:204`
- [ ] `.lspt` cases for a direct, a transitive, a recursive and an "at least" count, a test count read from a report and a "static" one, a client test for the click, and `rule:ide/five-features-are-one-reference-index` rewritten where it says no call-site edges exist. The rule `shipped`.

### 2. call hierarchy walks the call graph both ways

**Does:** `textDocument/callHierarchy` answers incoming and outgoing calls from the call graph stage 1
built.

- [ ] The server declares `callHierarchyProvider`, and `crates/nvs-lsp/tests/handshake.rs` holds the eighteenth standard request (`rule:ide/the-request-set-is-closed`). `crates/nvs-lsp/src/capabilities.rs:297`, `crates/nvs-lsp/tests/handshake.rs:161`
- [ ] `prepare` resolves the function or method under the cursor, `incomingCalls` lists the functions whose occurrences resolved to it, and `outgoingCalls` lists the functions it calls, each with the call ranges. `crates/nvs-lsp/src/index.rs:271`
- [ ] A call through a `callable` value or a `mixed` receiver is not shown, as in the lens.
- [ ] A Rust test for both directions over a recursive pair and a call through a `callable`, and `rule:ide/the-request-set-is-closed` rewritten from "admitted" to served.

### 3. a route's hover shows the program's grants

**Does:** hovering a `#[Route]` or its handler's name lists what the program's grants allow, and names
the `nvs.toml` it read.

- [ ] The hover resolves the snapshot for the program's entry file the way the extension set does, once per hover (`rule:ide/a-route-hover-shows-the-programs-grants`). `crates/nvs-lsp/src/document.rs:787`, `crates/nvs-config/src/snapshot.rs:149`
- [ ] It lists the grants by kind with their scopes, under a first line naming the file and saying the grants are the program's. `crates/nvs-config/src/tree.rs:303`, `crates/nvs-lsp/src/hover.rs:132`
- [ ] With no `nvs.toml`, it says so and names the default policy.
- [ ] `.lspt` cases over a fixture with grants, a changed grant giving a changed hover with no source edit, and none. The rule `shipped`.

### 4. a `tainted` or `secret` value's hover shows its path

**Does:** under a switch only `nvs-lsp` sets, the checker records where a qualifier entered a method and
each step it took, and the hover shows it.

- [ ] A switch on the expression table that only `nvs-lsp` sets, and a test that `nvs check` and a run leave it off (`rule:ide/a-hover-shows-where-a-tainted-or-secret-value-came-from`). `crates/nvs-types/src/expr_table.rs:1413`, `crates/nvs-types/src/check.rs:128`, `crates/nvs-lsp/src/document.rs:741`
- [ ] Under it, the checker records the source, each assignment and concatenation that carried the qualifier, and the launderer. `crates/nvs-types/src/expr/quals.rs:427`, `crates/nvs-types/src/expr/quals.rs:595`, `crates/nvs-types/src/expr/quals.rs:655`
- [ ] Hover on the value, or on an argument at a sink, shows the steps with their lines and the launderer marked. `crates/nvs-lsp/src/hover.rs:132`
- [ ] `.lspt` cases for a `tainted` and a `secret` value from source to sink, one laundered on the way, and one that entered through a declared parameter. The rule `shipped`.

### 5. a query call passes its site id, and a traced span records the site

**Does:** each `Core\Db` call that sends a statement gets a constant site id, and a traced span carries
the file and line it names.

- [ ] A roster of the `Core\Db` members that send a statement, on the pattern `CALL_SITE_MEMBERS` and `PATH_GRAPH_MEMBERS` use, and the rule fixes the argument's slot (`rule:observability/a-query-span-names-its-call-site`). `crates/nvs-stdlib/src/registry.rs:3749`, `crates/nvs-stdlib/src/registry.rs:3833`
- [ ] The lowering passes the id at each call, the callable-reference thunk passes zero, and the program keeps a site table: file, line and the file's text hash. `crates/nvs-ir/src/lower/anon_fn.rs:811`
- [ ] The span records the site only when it is recorded. `crates/nvs-stdlib/src/db/span.rs:51-56`, `crates/nvs-stdlib/src/db/execute.rs:420`, `crates/nvs-db/src/span.rs:91`
- [ ] Tests: two calls with the same SQL are two sites, an untraced call reads no site, and `a_query_span_contains_no_parameter_value_anywhere` still passes. The rule `shipped`.

### 6. each query call shows what the newest recorded request ran there

**Does:** with `[debug] store` on, a lens above each query call reads the debug index for the newest
recorded request that reached the site.

- [ ] The server opens the debug index read-only from the data folder, and an index that is absent or locked gives no lens (`rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`). `crates/nvs-config/src/data.rs:119`
- [ ] The lens shows the count, the total and the slowest time at the site, and "repeated" with the count when one statement ran there more than once. Its click opens the request in the viewer. `crates/nvs-lsp/src/server.rs:1173`
- [ ] A record whose file hash is not the file on disk is not shown, and the viewer's compare aligns two requests' queries by site, which `debug-viewer` leaves to this goal.
- [ ] A served fixture that runs one query in a loop of many iterations shows one lens at that site with the right count, flagged as repeated, and its record does not grow with the iteration count. The rule `shipped`.

### 7. a recorded request becomes a test that asserts its response

**Does:** `nvs debug to-test <trace-id>` prints a `Core\Test::request` case for a recorded request that
asserts the recorded response.

- [ ] The command reads the request and its response from the debug index and prints one class with one `#[Test]` method, through the formatter's printer (`rule:testing/a-recorded-request-becomes-a-test`). `crates/nvs-stdlib/src/test.rs:739`
- [ ] The method asserts the status, `Content-Type`, `Location` on a redirect, and the body: a JSON body through `json()` compared structurally, any other body as bytes. A body the spool cut at `request_body` gives a test that asserts the status and the headers only.
- [ ] A header the record holds as `[redacted]` is written as the placeholder with its comment, and a request whose recorded request body is not the whole body is refused with the reason (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`).
- [ ] `--write <file>` writes a new file and never replaces one, and the viewer's request view gains the button that shows the printed test with a copy button.
- [ ] A served fixture: the generated test passes against the unchanged program, and fails when its handler's status, its `Content-Type` or one JSON field of its body changes. The help text, the feature's proofs once the roster names it (`bun nv proofs --id`), the rule `shipped`, and `bun nv owners --closes request-aware-editor` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written. Stage 1 needs `editor-fixes` stage 4, which gives a lens its command. Stages 6 and 7 need the
spool and the index from `debug-service`.

### Next

Stage 1, the call graph, on `crates/nvs-lsp/src/index.rs` and `crates/nvs-lsp/src/server.rs`:
- record each occurrence's enclosing function. `crates/nvs-lsp/src/index.rs:1183`
- walk callers from a method to route handlers and tests. `crates/nvs-types/src/routes.rs:339`
- the lens title and its `nvs.runTests` command. `crates/nvs-lsp/src/server.rs:1173`

### Backlog

- A test count read from a coverage report is as old as that run; the next coverage run refreshes it (`docs/rules/ide/used-by-counts-the-routes-and-tests-that-reach-a-method.md`).
