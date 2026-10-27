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

- [ ] An example that reads a typed query parameter and shows the `400` for a bad value, and the feature's `about.md` says what a query parameter receives. `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/07-a-query-parameter-and-a-bad-value.nvs:1`, `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/about.md:1`
- [ ] The attack: a request sends a repeated key, an overlong number and a bad escape to a typed query parameter, and each is a `400` with the handler not run (`rule:testing/hostile-case-contract`). `tests/hostile/lang/attributes/core-route-and-core-access-the-route-table/a-query-value-that-does-not-convert.nvs:1`
- [ ] The reference's `#[Core\Query]` paragraph says the value is bound and a bad one is a `400`. `docs/reference/lang/90-attributes.md:997`
- [ ] The rule's "path half is built" paragraph rewritten to the present, and the record `shipped` (`rule:routing/a-bad-query-value-is-a-400`). `docs/rules/routing/a-bad-query-value-is-a-400.md:20`, `data/rules/routing/a-bad-query-value-is-a-400.json:3`

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

Stages 1 and 2 are done. `Core\Router::dispatch` binds each `#[Query]` parameter from the query
string, an absent optional key from its default, and answers `400` (`BadCapture`, told the match) for
a value that does not convert, a missing required key, a repeated key and a bracketed list. Four Rust
tests in `router.rs`'s `dispatching` module and the conformance case
`tests/conformance/core/a-query-value-that-does-not-convert-is-a-400.nvst` pass, and the case is in
the rule's `guardedBy`. The rule fragment's last paragraph says what is built; its status is still
`designed`. The `BadCapture` card and `docs/examples/types/Router-Refusal/about.md` name the query.

The acceptance sweep's reds were already fixed on disk: `crypto-bigint` is 0.7.10 (c244b4b36), the
count test runs in one folder, and `env-secret-proofs` exits 0 when rerun.

### Next

**Stage 3**, as its `**Split:**` line says. No gate.

- [ ] Example 07 in the route-table feature, reading a typed query parameter and showing the `400`. `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/02-captures-that-convert-and-a-query-parameter.nvs:1`, `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/about.md:1`
- [ ] The attack under `tests/hostile/lang/attributes/core-route-and-core-access-the-route-table/`: a repeated key, an overlong number, a bad escape. `tests/hostile/lang/attributes/an-attribute-is-an-anonymous-object-attached-to-a-declaration:1`
- [ ] The reference's `#[Core\Query]` paragraph says the value is bound and a bad one is a `400`. `docs/reference/lang/90-attributes.md:1072`
- [ ] `examples/parses.nvs` still says nothing binds a query parameter; rewrite it to bind `#[Core\Query] Slug $tag`. `examples/parses.nvs:98`
- [ ] The rule record `shipped`. `data/rules/routing/a-bad-query-value-is-a-400.json:3`
- [ ] `owners-closes`: the gap record `data/gaps/nvs-ir/a-callable-reference-to-a-core-member-with-an-options-bag.json` names this goal; close it or give it another owner. `data/gaps/nvs-ir/a-callable-reference-to-a-core-member-with-an-options-bag.json:5`

### Backlog

- Stage 4's gate. Expect `website/public/llms-full.txt` to need a re-render for the `BadCapture` card,
  and the Rust suites of `nvs-types`, `nvs-runtime` and `nvs-stdlib` to stay green; no test is
  expected to go red.
