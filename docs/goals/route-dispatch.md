---
looper: 1
title: "A matched route calls its own method, and one gate answers around it"
summary: "Core\\Router::dispatch calls the matched method, every refusal a program can answer reaches its gate, and both example apps use it."
context: [docs/rules/routing/matching-is-not-dispatching.md, docs/rules/routing/a-match-is-not-invocable.md, crates/nvs-stdlib/src/router.rs]
stages:
  - id: 1
    title: the rules
    status: done
    context: [docs/rules/routing/the-servers-match-dispatches-nothing.md, docs/rules/security/access-is-checked-for-presence-not-meaning.md, docs/rules/security/csrf-is-on-by-default.md, docs/rules/programs/framework-web-package.md, docs/rules/testing/in-process-request.md]
    extra:
      context:
        rules: [routing/matching-is-not-dispatching, routing/a-match-is-not-invocable, security/access-is-checked-for-presence-not-meaning]
        shapes: [A rule fragment]
  - id: 2
    title: "`Core\\Router::dispatch` and the gate, for what the program decides"
    status: done
    context: [crates/nvs-types/src/routes.rs, crates/nvs-types/src/links.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-stdlib/src/registry.rs, data/spec/core-members.json]
    extra:
      context:
        rules: [routing/matching-is-not-dispatching, security/route-capture-is-laundered-by-its-type, core-api/reference-card]
        shapes: ["A `Core` member", "A `.nvst` test case", A diagnostic]
  - id: 3
    title: "the server's refusals reach the gate"
    status: done
    context: [crates/nvs-cli/src/serve.rs, crates/nvs-cli/src/runner.rs, crates/nvs-server/src/route.rs, crates/nvs-runtime/src/ctx/inbound.rs]
    extra:
      context:
        rules: [security/csrf-is-on-by-default, http-server/the-body-is-read-on-demand-under-two-caps, testing/in-process-request]
        shapes: ["A `.nvst` test case"]
  - id: 4
    title: "`failed` after a memory or time limit"
    status: done
    context: [crates/nvs-server/src/serve.rs]
    extra:
      context:
        rules: [http-server/a-requests-blast-radius-is-bounded-at-four-tiers]
        shapes: []
  - id: 5
    title: the example apps use the gate
    status: done
    context: [apps/notes/src/App.nvs, apps/notes/public/index.nvs, apps/paperwork/src/App.nvs, apps/paperwork/public/index.nvs, website/src/content/docs/guides/example-apps/notes.mdx, website/src/content/docs/guides/example-apps/paperwork.mdx, tools/nv/cmd/site.ts]
    extra:
      context:
        rules: []
        shapes: ["An example"]
  - id: 6
    title: the website teaches dispatch
    status: open
    context: [website/src/content/docs/syntax/attributes/routes.mdx, website/src/content/docs/guides/how-to-use/requests.mdx, website/src/content/docs/guides/simple-programs/json-api.mdx, docs/examples/lang/attributes/core-route-and-core-access-the-route-table/about.md]
    extra:
      context:
        rules: []
        shapes: ["An example"]
  - id: 7
    title: the feature proofs
    status: open
    extra:
      context:
        rules: [testing/feature-proofs]
        shapes: ["An example", "A `.nvst` test case"]
  - id: 8
    title: a match reads another attribute of its route
    status: open
    context: [crates/nvs-stdlib/src/router.rs, crates/nvs-runtime/src/routes.rs]
    extra:
      context:
        rules: [routing/a-match-is-not-invocable, core-classes/derive-attribute]
        shapes: ["A `Core` member", "An example"]
checks:
  - id: rules
    stage: 1
    name: the rulebook is consistent
    run: [bun, nv, rules, --check]
  - id: dispatch-cases
    stage: 2
    name: "dispatch calls the matched method, and each program-side refusal reaches the gate"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/router-dispatch-calls-the-matched-method.nvst, --case, tests/conformance/core/router-dispatch-refusals-reach-the-gate.nvst, --case, tests/conformance/core/router-dispatch-discards-a-handlers-return-value.nvst]
  - id: door-refusals
    stage: 3
    name: "CSRF, body-too-large and unknown-method refusals reach a dispatching program's gate, and never a handler"
    run: [cargo, test, --test, gate]
    tests: [a_csrf_refusal_reaches_the_gate_and_no_handler, a_body_over_the_cap_reaches_the_gate_and_no_handler, an_unknown_method_reaches_the_gate_and_no_handler, a_gate_that_throws_leaves_the_empty_answer, a_program_that_does_not_dispatch_keeps_the_empty_answer]
  - id: limit-failed
    stage: 4
    name: "a program stopped by its memory or time limit reaches `failed`"
    run: [cargo, test, --test, gate]
    tests: [a_program_over_its_memory_limit_reaches_failed, a_program_over_its_time_limit_reaches_failed]
  - id: apps
    stage: 5
    name: "the example apps and their pages are current"
    run: [bun, nv, site, --check, apps]
  - id: guides
    stage: 6
    name: "the guides and their snippets run"
    run: [bun, nv, site, --check, guides]
  - id: snippets
    stage: 6
    name: "every snippet runs"
    run: [bun, nv, site, --check, snippets]
  - id: dispatch-proofs
    stage: 7
    name: "`Core\\Router::dispatch` owes nothing"
    run: [bun, nv, proofs, --gate, --only, "Core\\Router::dispatch"]
  - id: attribute-proofs
    stage: 8
    name: "`Core\\Router\\Match::attribute` owes nothing"
    run: [bun, nv, proofs, --gate, --only, "Core\\Router\\Match::attribute"]
---

# A matched route calls its own method, and one gate answers around it

## Why

A program with routes writes its own dispatcher today. `Core\Request::route()` returns a match, and
the program compares `$match->name()` against every route name to call the method
(`apps/notes/src/App.nvs:16`). The list repeats every `#[Route]`, and nothing checks it.

- A route with no arm matches and runs nothing.
- A dispatcher that never reads `accessAs` serves every route unguarded.

`rule:security/access-is-checked-for-presence-not-meaning` states that second limit itself.

The rules leave dispatch to a framework that generates the `switch`
(`rule:routing/a-match-is-not-invocable`). That framework, `nvs/web`, is not on disk. So every
program, both example apps included, writes the list by hand.

The server's own refusals are empty answers that no program sees: CSRF `403`, body over the cap
`413`, unknown method `501` (`crates/nvs-cli/src/serve.rs:927`, `:1077`, `:1094`). So an app can
show no page for the most common of them: a form sent after its session expired.

## Standing decisions

The user decided this design interactively. A session does not reopen it.

**`Core\Router::dispatch(Core\Router\Gate $gate): void` calls the matched method.** The compiler
generates the jump from the route table it already builds (`crates/nvs-types/src/routes.rs:613`).
The jump is indexed by the matched row, and nobody writes a list. The handler label still never
crosses into a program (`crates/nvs-runtime/src/routes.rs:54`), and `Core\Router\Match` stays
not invocable.

**The gate is an interface with three methods.**

```nvs
interface Core\Router\Gate {
    public function allows(Core\Router\Match $route): bool;
    public function refused(Core\Router\Refusal $reason, ?Core\Router\Match $route): void;
    public function failed(?Core\Router\Match $route, ?Throwable $error): void;
}
```

- `allows` runs before every handler. The handler runs only when it returns `true`. A `false` or a
  throw is `Refusal::Denied`.
- `dispatch` sets the status before it calls `refused` or `failed`. A hook that only renders a page
  sends the right code, and a hook may set another.
- `Core\Router\Refusal` has the cases `NotFound` (404), `MethodNotAllowed` (405 with `Allow`, from
  `Core\Router::methodsFor`), `UnknownMethod` (501), `Denied` (403), `Csrf` (403), `BodyTooLarge`
  (413) and `BadCapture` (400, a `Parses` capture whose `parse` threw, from
  `crates/nvs-stdlib/src/router.rs:1822`).
- A new server refusal later is a new case. Whether `match` already reports a missing case is not
  checked, and stage 2 finds out.

**The gate answers a refusal and never overturns one.** It sets the page, the status, headers and a
log line, and the handler never runs. The only opt-out stays `csrf: false` on the route
(`rule:security/csrf-is-on-by-default`).

**The server's refusals reach the gate in refusal mode.** The door still decides exactly as today.
For a refused request it runs the entry anyway, under these conditions:

- `Core\Request::route()` returns `null`, so no dispatcher, Core or hand-written, can reach a handler.
- There is no body.
- `dispatch` reads the refusal off the carrier and calls `refused`.
- A gate that throws leaves today's empty answer, never the handler.

The unknown-method check moves to after the mount is chosen (`crates/nvs-cli/src/serve.rs:927`),
which changes nothing else because a mount is chosen by path. The in-process door
(`crates/nvs-cli/src/runner.rs:1595`) does the same, so `Core\Test::request` tests it.

**Only a program that calls `Core\Router::dispatch` gets refusal mode.** The compiler sees the call,
as it sees `Core\Router::url` (`crates/nvs-types/src/links.rs`), and marks the unit. A program that
dispatches by hand keeps today's empty answers, because it would read a refused request as an
ordinary one with no route. `Core\Request::route()` and `Core\Router::match` stay for it, unchanged.

**`failed` covers a program stopped by its memory or time limit,** in a fresh isolate with the same
limits. If that one fails too, the answer is today's empty `500`. How a limit reports its stop is
not checked yet, and stage 4 starts there.

**A handler may declare any return type, and `dispatch` discards the value.** The user chose this over
a `void`-only rule, which would remove the response schema and `#[Api]` example that `nvs build
--openapi` reads from the return type (`crates/nvs-cli/src/openapi.rs:196`). A handler is a static
method, or a method of a class with a no-argument constructor. Anything else is a compile error at the `#[Route]`. Constructor injection and a
return-value-to-response rule are out of this goal.

**Out of the gate, each for its reason:**

- The CORS preflight (`crates/nvs-server/src/serve.rs:1515`). Browsers never show its body, so a
  page there costs a program run for nothing.
- Malformed HTTP, and an unreadable `X-Forwarded-For` (`serve.rs:1466`). There is no request yet, or
  the reader is a proxy.
- The socket-and-event-stream conflict (`serve.rs:1902`). The request has already ended.
- The capacity `503` (`serve.rs:1499`), the drain `503` (`serve.rs:1832`) and a path no mount
  covers (`crates/nvs-cli/src/serve.rs:945`). No code may run there, and static pages from the
  configuration are a later goal.

**Trade-offs, stated once:**

- Performance:
  - The jump replaces a chain of string compares.
  - CSRF, body-too-large and unknown-method refusals now start the program. That costs what a
    `GET` to any page costs, and the load-shedding paths stay free.
- Memory:
  - One generated entry per route, per program.
  - Nothing per request beyond the isolate the request already has.
- Security: unchanged decisions, plus one place where `#[Access]` is always enforced.
- Usability: no list to keep, and every error page in one class.

**The website teaches `dispatch` first.** Hand-written dispatch stays a valid, documented way, shown
after it. Every page and comment this goal writes follows AGENTS.md § *Text an end user reads*.

## Stages

### 1. the rules

**Does:** the rulebook states the design above before any code follows it.

- [x] Rewrite the dispatch decline as dispatch through a gate (`docs/rules/routing/matching-is-not-dispatching.md:1`)
- [x] Keep the match not invocable, and drop its "the ergonomic cost is paid by the framework" paragraph (`docs/rules/routing/a-match-is-not-invocable.md:28`)
- [x] The server still calls nothing, and the program calls `dispatch` (`docs/rules/routing/the-servers-match-dispatches-nothing.md:1`)
- [x] `dispatch` is the dispatch site the limit paragraph says is missing (`docs/rules/security/access-is-checked-for-presence-not-meaning.md:24`)
- [x] The refusal reaches a dispatching program's gate (`docs/rules/security/csrf-is-on-by-default.md:1`)
- [x] `Web\Controller` no longer owns dispatch (`docs/rules/programs/framework-web-package.md:7`)
- [x] A failing program answers through `failed` (`docs/rules/testing/in-process-request.md:26`)
- [x] A new fragment for the gate, `Refusal` and refusal mode (`docs/rules/routing/a-gate-answers-around-every-handler.md:1`)

### 2. `Core\Router::dispatch` and the gate, for what the program decides

**Does:** `dispatch` calls the matched method and sends `NotFound`, `MethodNotAllowed`, `Denied`,
`BadCapture` and a handler's throw to the gate.

- [x] `Core\Router::dispatch`, `Core\Router\Gate` and `Core\Router\Refusal`, with reference cards (`crates/nvs-stdlib/src/router.rs:2354`)
- [x] The spec rows (`data/spec/core-members.json:1`, `docs/spec/01-core-library.md:1`)
- [x] The jump over the route table: the matched row's handler, resolved through its class (`crates/nvs-stdlib/src/router.rs:2521`)
- [x] The unit's "dispatches" mark, found as links are (`crates/nvs-types/src/links.rs:146`)
- [x] The no-argument-constructor handler error, `E0855` (`crates/nvs-types/src/routes.rs:778`)
- [x] The three conformance cases the `dispatch-cases` check names: two are written, and the third pins that a handler's return value is discarded (`tests/conformance/core/router-dispatch-discards-a-handlers-return-value.nvst:1`)

### 3. the server's refusals reach the gate

**Does:** CSRF, body-too-large and unknown-method refusals reach a dispatching program's gate in
refusal mode, and never a handler.

- [x] The refusal on the carrier, and `route()` returning `null` under it (`crates/nvs-runtime/src/ctx/inbound.rs:716`)
- [x] The served door: unknown method after mount selection, and CSRF and the body cap as refusal mode (`crates/nvs-cli/src/serve.rs:927`)
- [x] The in-process door, the same (`crates/nvs-cli/src/runner.rs:1595`)
- [x] The Rust tests the `door-refusals` check names (`crates/nvs-cli/tests/gate.rs:1`)

### 4. `failed` after a memory or time limit

**Does:** a program stopped by a limit gets its `failed` page from a fresh isolate.

- [x] Find how a limit's stop reaches the door (`crates/nvs-server/src/serve.rs:1840`)
- [x] Run the gate's `failed` in a fresh isolate under the same limits, with the empty `500` as fallback (`crates/nvs-server/src/serve.rs:2444`)
- [x] The two Rust tests the `limit-failed` check names (`crates/nvs-cli/tests/gate.rs:1`)

### 5. the example apps use the gate

**Does:** neither example app has a hand-written dispatcher.

- [x] Notes: delete `App::handle`, add `src/Gate.nvs` with its error pages, and make `index.nvs` one `dispatch` call (`apps/notes/src/App.nvs:16`, `apps/notes/public/index.nvs:6`)
- [x] Notes: the tests cover a 404, a 405, a CSRF refusal and a failure page (`apps/notes/src/AppTest.nvs:1`)
- [x] Paperwork: the same (`apps/paperwork/src/App.nvs:16`, `apps/paperwork/public/index.nvs:6`)
- [x] Both pages describe the gate and list `Gate.nvs`, and the copies under `website/public/example-apps/` are rewritten (`website/src/content/docs/guides/example-apps/notes.mdx:98`, `website/src/content/docs/guides/example-apps/paperwork.mdx:143`, `tools/nv/cmd/site.ts:28`)

### 6. the website teaches dispatch

**Does:** every page that shows routes shows `dispatch` and a gate first.

- [ ] The routes syntax page and its match snippet (`website/src/content/docs/syntax/attributes/routes.mdx:54`, `website/snippets/syntax/routes/02-match.nvs:26`)
- [ ] The requests guide and its snippet (`website/src/content/docs/guides/how-to-use/requests.mdx:16`, `website/snippets/guides/how-to-use/requests/requests.nvs:12`)
- [ ] The JSON API guide and its snippet (`website/src/content/docs/guides/simple-programs/json-api.mdx:26`, `website/snippets/guides/simple-programs/json-api/json-api.nvs:33`)
- [ ] The route-test snippet (`website/snippets/guides/testing/test-a-route/test-a-route.nvs:16`)
- [ ] The route-table feature's description (`docs/examples/lang/attributes/core-route-and-core-access-the-route-table/about.md:1`)
- [ ] The hand-written examples still read true beside `dispatch` (`docs/examples/core/Router-Match/name/03-call-the-method-for-each-route.nvs:1`, `docs/examples/core/Router-Match/accessAs/03-one-check-before-every-handler.nvs:1`, `docs/examples/core/Router/methodsFor/03-choose-between-404-and-405.nvs:1`)

### 7. the feature proofs

**Does:** `Core\Router::dispatch` owes nothing (`bun nv proofs --id 'Core\Router::dispatch'`).

- [ ] Its `about.md` and three examples (`docs/examples/core/Router/dispatch/about.md:1`)
- [ ] The bench, with its growth against the number of routes (`benches/members/core/Router/dispatch.nvs:1`)
- [ ] The attack: a refused request tries to reach a handler, including through a gate that throws (`tests/hostile/core/Router/dispatch/01-a-refused-request-tries-to-reach-a-handler.nvs:1`)
- [ ] Its help in the binary (`crates/nvs-stdlib/src/router.rs:207`)

### 8. a match reads another attribute of its route

**Does:** a gate can read a per-route option such as `#[Cache]` or `#[RateLimit]` off the match.

- [ ] `Core\Router\Match::attribute<T>()`, typed by a shape the way `Core\Program::implementingWith` is, and still data rather than something callable (`crates/nvs-stdlib/src/router.rs:560`)
- [ ] Its rule, beside the match rule (`docs/rules/routing/a-match-is-not-invocable.md:1`)
- [ ] Its feature proofs (`docs/examples/core/Router-Match/attribute/about.md:1`)

## Handoff

### State

Stages 2 to 5 are done, and `rule:routing/a-gate-answers-around-every-handler` is `shipped`.
`door-refusals` and `limit-failed` pass: `cargo test --test gate`, eight tests over a real `nvs serve`.
`apps` passes with the current binary: `bun nv site --check apps --nvs target/debug/nvs.exe`.

- Both example apps call `Core\Router::dispatch(new <App>\Gate())` from `public/index.nvs`, and their
  `src/Gate.nvs` checks `Audience::Public` and renders the error pages. Notes' tests cover a 404, a
  405, a CSRF `403` and a failure `500` (a note number past SQLite's `INTEGER`). Paperwork has no
  unsafe route and nothing that throws, so its tests cover the 404 and the 405.
- `dispatch` opts a program into the autoload scan, as `route()` does (`crates/nvs-hir/src/requires.rs:853`,
  `rule:routing/table-is-opt-in`). Without it an entry that names only its gate compiled no route.
- `Core\Test::request` takes the CSRF refusal for a dispatching unit (`crates/nvs-cli/src/runner.rs:642`),
  pinned by `tests/conformance/core/router-dispatch-a-test-requests-csrf-refusal-reaches-the-gate.nvst`.
- The carrier's `nvs_runtime::routes::Refusal` has `UnknownMethod`, `Csrf`, `BodyTooLarge` and
  `Limit` (`crates/nvs-runtime/src/routes.rs:707`). `Inbound::route()` is `None` while one is set.
- A limit's stop is recorded by `Ctx::run_limit_handler` (`crates/nvs-runtime/src/ctx/hooks.rs:518`)
  and crosses as `Failure::limit` (`crates/nvs-runtime/src/host.rs:400`).
- The serving door returns `Reply::Gated` with a lazy `Rerun` for a dispatching unit's accepted
  request (`crates/nvs-cli/src/serve.rs:1122`, `crates/nvs-server/src/serve.rs:467`). The accept arm
  is a loop that runs it once when `Memory` or `CpuTime` stopped the first run
  (`crates/nvs-server/src/serve.rs:1942`). The rerun has no body, no upgrade offer, the first run's
  trace, and `Refusal::Limit`, so a failing rerun answers the empty `500`.
- `dispatch` answers `Refusal::Limit` with `failed(null, null)` and status `500`
  (`crates/nvs-stdlib/src/router.rs:2319`, `:2477`).
- The test listener's door (`crates/nvs-cli/src/runner.rs:1696`) still returns `Reply::Run`, so
  neither it nor `Core\Test::request` reruns after a limit.

Decisions from stage 2 still open to review: `BadCapture` before `allows` with a `null` route; a throw
from `allows` is `Denied`; a parameter no capture fills throws `LogicError`. From stage 4: `failed`
gets no route and no error after a limit; a run whose head was already streamed is not rerun.

### Next

Stage 6, the website's routing pages and their snippets under `website/` (`rule:routing/matching-is-not-dispatching`, § *Text an end user reads*):

- [ ] The routes syntax page and its match snippet (`website/src/content/docs/syntax/attributes/routes.mdx:54`, `website/snippets/syntax/routes/02-match.nvs:26`)
- [ ] The requests guide and its snippet (`website/src/content/docs/guides/how-to-use/requests.mdx:16`, `website/snippets/guides/how-to-use/requests/requests.nvs:12`)
- [ ] The JSON API guide and its snippet (`website/src/content/docs/guides/simple-programs/json-api.mdx:26`, `website/snippets/guides/simple-programs/json-api/json-api.nvs:33`)
- [ ] The route-test snippet (`website/snippets/guides/testing/test-a-route/test-a-route.nvs:16`)

The example apps' `src/Gate.nvs` (`apps/notes/src/Gate.nvs:11`) is the shape a snippet's gate can copy.

### Backlog

- Static error pages from the configuration for the capacity `503`, the drain `503` and a path no mount covers: a later goal (`crates/nvs-server/src/serve.rs:1499`).
- The CSRF token from a form field, since the door reads only `X-CSRF-Token` and a plain form cannot send it: a later goal (`crates/nvs-server/src/route.rs:128`).
- Constructor injection for handler classes: not decided (`docs/rules/routing/a-match-is-not-invocable.md:21`).
- Whether the generated jump binds `#[Query]` parameters and answers their `400`: not decided, M16 owns it today (`docs/rules/routing/a-bad-query-value-is-a-400.md:20`).
- The checker does not hold an implementing member's signature to the `Gate` interface's signature, and `$v is Core\Router\Gate` folds to `false` (`crates/nvs-types/src/conformance.rs:1`).
- `nvs-stdlib`'s ldap tests and `nvs-cli`'s `live_edit` fail beside the other test binaries and pass alone (`crates/nvs-stdlib/tests/ldap.rs:1485`, `crates/nvs-cli/tests/live_edit.rs:1010`).
- `Core\Test::request` does not rerun a dispatching program after a limit, and takes no CSRF refusal for a program that does not dispatch (`crates/nvs-cli/src/runner.rs:625`).
