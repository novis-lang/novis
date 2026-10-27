---
looper: 1
title: "A `#[Query]` parameter is bound from the query string, and a bad value is a `400`"
summary: "Core\\Router::dispatch fills every #[Query] parameter of a route method from the query string, a value that does not convert or a required key that is absent is a 400 before allows and the handler run, the feature has its proofs, and rule routing/a-bad-query-value-is-a-400 is shipped."
context: [docs/rules/routing/a-bad-query-value-is-a-400.md, docs/rules/routing/a-query-parameter-is-declared-like-a-capture.md]
stages:
  - id: 1
    title: "`dispatch` fills a `#[Query]` parameter from the query string"
    status: done
    context: [crates/nvs-cli/src/main.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-stdlib/src/router.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/uri.rs]
    extra:
      context:
        rules: [routing/a-query-parameter-is-declared-like-a-capture, routing/a-bad-query-value-is-a-400, security/route-capture-is-laundered-by-its-type]
  - id: 2
    title: "a value that does not convert, or a missing required key, is a `400`"
    status: done
    context: [crates/nvs-stdlib/src/router.rs, tests/conformance/core/a-segment-the-class-refuses-never-reaches-the-handler.nvst, docs/rules/errors/ambiguous-input-refused.md]
    extra:
      context:
        rules: [routing/a-bad-query-value-is-a-400, errors/ambiguous-input-refused]
  - id: 3
    title: "the binding has its feature proofs, and the rule is shipped"
    status: open
    context: [docs/rules/routing/a-bad-query-value-is-a-400.md, data/rules/routing/a-bad-query-value-is-a-400.json, docs/examples/lang/attributes/core-route-and-core-access-the-route-table/about.md, docs/rules/testing/feature-proofs.md]
    extra:
      context:
        rules: [testing/feature-proofs, routing/a-bad-query-value-is-a-400, testing/hostile-case-contract]
  - id: 4
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: owners-closes
    stage: 3
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, query-binding]
    reach: true
    floor: false
---

# A `#[Query]` parameter is bound from the query string, and a bad value is a `400`

## Why

A route method can declare a `#[Query]` parameter, and the compiler checks it: the marker, its type
list and its optional bit (`rule:routing/a-query-parameter-is-declared-like-a-capture`). The API
document reads it too. But nothing delivers the value. The route table drops every query parameter
where it crosses into the runtime (`crates/nvs-types/src/crossing.rs:43`), so a handler reads
`Core\Request::query()` and converts the value itself. This goal finishes the feature that
`rule:routing/a-bad-query-value-is-a-400` specifies. It was owned by M16, which is gone: Novis ships no
framework (`rule:programs/no-first-party-framework`), and this binding is `Core\Router` work.

## Standing decisions

**The binding happens where a path capture's does.** `Core\Router::dispatch` converts each query key
by the same `CaptureConv` a path capture uses (`crates/nvs-cli/src/main.rs:1437`), at the point a
`Parses` capture is converted, so a bad query value is a `400` before `allows` runs, exactly like
`refusal::BAD_CAPTURE` (`crates/nvs-stdlib/src/router.rs:2527`). `Core\Router::match`'s result
stays the path's captures.

**A repeated key for a scalar parameter is a `400`.** Two values for one parameter is ambiguous input
(`rule:errors/ambiguous-input-refused`), so neither value is picked.

**The feature id is `lang:attributes/core-route-and-core-access-the-route-table`.** `#[Core\Query]` is
declared and documented in that feature's section (`docs/reference/lang/90-attributes.md:962`), and its
example 02 already declares a query parameter. The binding's proofs join that feature: no second heading
for one attribute, which keeps the reference simpler. Safe default if `bun nv proofs --id` names another
owner for the marker: add the proofs there and say so in the commit.

**The query is parsed once per dispatch, by the parser `Core\Request::query()` uses**
(`crate::uri::parse_query`, `crates/nvs-stdlib/src/uri.rs:3181`). One parser means one reading of a
bracket key or a bad escape, which is the security call; a second, faster parser is not written.

**An absent optional key's default crosses already converted.** `nvs_types::routes::RouteParam`
carries the folded default, and `crates/nvs-types/src/crossing.rs:179` (`query_default`) converts it
once at boot through the parameter's own `CaptureConv` into `nvs_runtime::routes::QueryParam::default`.
So a default and a written value are one value, and no constant lattice crosses into the runtime.

**A query refusal tells `refused` the match.** A path capture's `400` hands `refused` a `null` route,
because no match was built. A query value fails after the match exists, so the gate receives it. A
query value is decoded once, by the query parser, and never again by `decode_capture`.

**Lanes.** One lane: stages 1 and 2 share `crates/nvs-stdlib/src/router.rs` and run as one group in
one session. Stage 3 follows them, because its examples print what the binding returns.

**Trade-offs.** A program gains: a handler reads a typed, unqualified parameter instead of converting
`tainted` text itself. The query string is parsed only for a route method that declares a `#[Query]`
parameter, so a route without one costs nothing more. Memory is the converted values, per request, freed
with it. Simplicity of the language gains: a declaration that compiled and did nothing now does what it
says.

**The options-bag gap is fixed in this goal, by the thunk passing the bag's defaults.** No goal on
the chain lowers callable references, and a compiler panic on a program `nvs check` accepts is not
handed on. The thunk takes the parameters before the bag and forwards the call as a direct call that
omits the bag does. That adds no diagnostic and no language rule, so the surface stays simpler than a
checker refusal; a caller cannot pass a bag through the reference. The cost is nothing at run time.

## Stages

### 1. `dispatch` fills a `#[Query]` parameter from the query string

**Does:** a route method's `#[Query]` parameter receives the key's value, converted to its declared
type. An absent optional key gives the parameter's default.

- [x] The crossing carries each `ParamIn::Query` parameter with its `CaptureConv` beside the path captures, in a list of its own on `Route` (`rule:routing/a-query-parameter-is-declared-like-a-capture`). `crates/nvs-types/src/crossing.rs:43`, `crates/nvs-runtime/src/routes.rs:278`
- [x] `dispatch` reads the query once through `crate::uri::parse_query`, only for a row that declares a query parameter, and converts each key by `Route::convert`'s table. `crates/nvs-stdlib/src/router.rs:2752`, `crates/nvs-stdlib/src/request.rs:1867`
- [x] `call_handler` fills a query parameter from the converted value, and an absent optional key from the parameter's default. `crates/nvs-stdlib/src/router.rs:2823`
- [x] Rust tests beside the router's: a converted `uint`, an enum, a `Parses` class, and an absent optional key that gives the default. `crates/nvs-stdlib/src/router.rs:4798`

### 2. a value that does not convert, or a missing required key, is a `400`

**Does:** the request is answered `400`, and neither `allows` nor the handler runs. A conformance case
pins it beside the path capture's `404`, as the rule asks.

- [x] A value that does not convert, a required key that is absent, and a repeated key for a scalar parameter each refuse with `refusal::BAD_CAPTURE` before `allows` runs (`rule:routing/a-bad-query-value-is-a-400`, `rule:errors/ambiguous-input-refused`). `crates/nvs-stdlib/src/router.rs:2780`
- [x] A conformance case that asserts a path capture's `404` and a query value's `400` on one signature side by side, and that neither `allows` nor the handler ran, named in the rule's `guardedBy`. `tests/conformance/core/a-query-value-that-does-not-convert-is-a-400.nvst:1`, `data/rules/routing/a-bad-query-value-is-a-400.json:8`

### 3. the binding has its feature proofs, and the rule is shipped

**Does:** `bun nv proofs --id 'lang:attributes/core-route-and-core-access-the-route-table'` prints
nothing owed, and the rule's record is `shipped` with its cases in `guardedBy`.

**Split:** a writing subagent writes the example and the attack after stages 1 and 2 have landed; the
session checks both and writes the rule flip.

- [x] An example that reads a typed query parameter and shows the `400` for a bad value, and the feature's `about.md` says what a query parameter receives. `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/07-a-query-parameter-and-a-bad-value.nvs:1`, `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/about.md:1`
- [x] The attack: a request sends a repeated key, an overlong number and a bad escape to a typed query parameter, and each is a `400` with the handler not run (`rule:testing/hostile-case-contract`). `tests/hostile/lang/attributes/core-route-and-core-access-the-route-table/02-a-query-value-that-does-not-convert.nvs:1`
- [x] The reference's `#[Core\Query]` paragraph says the value is bound and a bad one is a `400`; it is written in the feature's `details.md`, which `bun nv render` turns into the chapter. `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/details.md:19`
- [x] The rule's "path half is built" paragraph rewritten to the present, and the record `shipped` (`rule:routing/a-bad-query-value-is-a-400`). `docs/rules/routing/a-bad-query-value-is-a-400.md:20`, `data/rules/routing/a-bad-query-value-is-a-400.json:3`
- [x] `examples/parses.nvs` binds `#[Core\Query] Slug $tag` through `Core\Router::dispatch`, and no comment says the binding is missing. `examples/parses.nvs:52`
- [ ] A callable reference to a `Core` member with an options bag lowers instead of panicking, and its gap record is deleted (see Standing decisions). `crates/nvs-ir/src/lower/anon_fn.rs:673`, `data/gaps/nvs-ir/a-callable-reference-to-a-core-member-with-an-options-bag.json:1`
  - [ ] The thunk's parameters stop before the bag, and its forwarded call passes the bag's defaults the way a direct call that omits the bag does. `crates/nvs-ir/src/lower/call.rs:203`
  - [ ] `crates/nvs-ir/tests/query_sites.rs` gains the case where the thunk passes the zero site id. `crates/nvs-ir/tests/query_sites.rs:1`
  - [ ] A conformance case calls `Core\Str::split(...)` through the reference. `tests/conformance/core/a-callable-reference-to-a-member-with-options-uses-the-defaults.nvst:1`
  - [ ] The gap record is deleted, and `bun nv owners --closes query-binding` exits 0. `data/gaps/nvs-ir/a-callable-reference-to-a-core-member-with-an-options-bag.json:1`

### 4. the goal is verified once, and every failure is fixed

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

Stages 1 and 2 are done, and stage 3's proofs have landed. The route-table feature has example 07
(a required `'name'|'price'` and an optional `uint` query parameter, `200` and `400`) and the attack
`02-a-query-value-that-does-not-convert.nvs` (4200 bad targets, each a `400` with the method not run).
`about.md` says it in one sentence, because its bound is 200 words. `details.md` lists the four
`400` cases. `bun nv render` and `bun nv render --website` wrote the chapter and `llms-full.txt`.
`examples/parses.nvs` binds `#[Core\Query] Slug $tag` through `dispatch`. The rule is `shipped`. The
attributes chapter's seven perf records were re-recorded, because the chapter changed. Their counts
match the old records. The clock figures are noise from a loaded machine, and the clock decides
nothing. `bun nv proofs --gate` over the chapter's nine features owes nothing.

A bad percent escape such as `%zz` reaches a `string` parameter as the literal text, with `200`: the
query parser is lenient, the same as `Core\Request::query()`. Invalid UTF-8 after decoding is a
`400`. The attack sends bad escapes only to the `uint` parameter. No rule claims otherwise.

`bun nv owners --closes query-binding` still names the options-bag gap, the last stage 3 item.

### Next

**Stage 3's last item**, by the standing decision: the thunk passes the bag's defaults. First check
what type the checker gives `Core\Str::split(...)`. If that closure type includes the bag as a
parameter, the thunk's arity must match it, so drop the bag there too
(`crates/nvs-types/src/expr/calls.rs`, grep `first_class`/`callable`).

- [ ] The thunk's parameters stop before a `CheckedTy::CoreShape` parameter, and the forwarded call passes the bag's defaults as a direct call that omits the bag does. `crates/nvs-ir/src/lower/anon_fn.rs:673`, `crates/nvs-ir/src/lower/call.rs:203`, `crates/nvs-ir/src/lower/mod.rs:5165`
- [ ] `crates/nvs-ir/tests/query_sites.rs` gains the thunk case with the zero site id. `crates/nvs-ir/tests/query_sites.rs:1`
- [ ] A conformance case calls `Core\Str::split(...)` through the reference. `tests/conformance/core/a-callable-reference-to-a-member-with-options-uses-the-defaults.nvst:1`
- [ ] The gap record is deleted, and `bun nv owners --closes query-binding` exits 0. `data/gaps/nvs-ir/a-callable-reference-to-a-core-member-with-an-options-bag.json:1`

Then set stage 3 `done`.

### Backlog

- Stage 4's gate. The Rust suites of `nvs-types`, `nvs-runtime`, `nvs-stdlib` and `nvs-ir`, and the
  `examples` test over `examples/parses.nvs`, should stay green. No test is expected to go red.
