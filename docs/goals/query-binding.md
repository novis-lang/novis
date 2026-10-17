---
looper: 1
title: "A `#[Query]` parameter is bound from the query string, and a bad value is a `400`"
summary: "Core\\Router::dispatch fills every #[Query] parameter of a route method from the query string, a value that does not convert or a required key that is absent is a 400 before allows and the handler run, the feature has its proofs, and rule routing/a-bad-query-value-is-a-400 is shipped."
context: [docs/rules/routing/a-bad-query-value-is-a-400.md, docs/rules/routing/a-query-parameter-is-declared-like-a-capture.md]
stages:
  - id: 1
    title: "`dispatch` fills a `#[Query]` parameter from the query string"
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-stdlib/src/router.rs, crates/nvs-stdlib/src/request.rs]
  - id: 2
    title: "a value that does not convert, or a missing required key, is a `400`"
    status: open
    context: [crates/nvs-stdlib/src/router.rs, docs/rules/errors/ambiguous-input-refused.md]
  - id: 3
    title: "the binding has its feature proofs, and the rule is shipped"
    status: open
    context: [docs/rules/routing/a-bad-query-value-is-a-400.md]
checks: []
---

# A `#[Query]` parameter is bound from the query string, and a bad value is a `400`

## Why

A route method can declare a `#[Query]` parameter, and the compiler checks it: the marker, its type
list and its optional bit (`rule:routing/a-query-parameter-is-declared-like-a-capture`). The API
document reads it too. But nothing delivers the value. The route table drops every query parameter
where it crosses into the runtime (`crates/nvs-cli/src/main.rs:2899`), so a handler reads
`Core\Request::query()` and converts the value itself. This goal finishes the feature that
`rule:routing/a-bad-query-value-is-a-400` specifies. It was owned by M16, which is gone: Novis ships no
framework (`rule:programs/no-first-party-framework`), and this binding is `Core\Router` work.

## Standing decisions

**The binding happens where a path capture's does.** `Core\Router::dispatch` converts each query key
by the same `CaptureConv` a path capture uses (`crates/nvs-cli/src/main.rs:2959`), at the point a
`Parses` capture is converted, so a bad query value is a `400` before `allows` runs, exactly like
`refusal::BAD_CAPTURE` (`crates/nvs-stdlib/src/router.rs:2513`). `Core\Router::match`'s result
stays the path's captures.

**A repeated key for a scalar parameter is a `400`.** Two values for one parameter is ambiguous input
(`rule:errors/ambiguous-input-refused`), so neither value is picked.

**Trade-offs.** A program gains: a handler reads a typed, unqualified parameter instead of converting
`tainted` text itself. The query string is parsed only for a route method that declares a `#[Query]`
parameter, so a route without one costs nothing more. Memory is the converted values, per request, freed
with it. Simplicity of the language gains: a declaration that compiled and did nothing now does what it
says.

## Stages

### 1. `dispatch` fills a `#[Query]` parameter from the query string

**Does:** a route method's `#[Query]` parameter receives the key's value, converted to its declared
type. An absent optional key gives the parameter's default.

### 2. a value that does not convert, or a missing required key, is a `400`

**Does:** the request is answered `400`, and neither `allows` nor the handler runs. A conformance case
pins it beside the path capture's `404`, as the rule asks.

### 3. the binding has its feature proofs, and the rule is shipped

**Does:** `bun nv proofs --id` for the feature prints nothing owed, and the rule's record is `shipped`
with its cases in `guardedBy`.

## Handoff

### State

The goal is new. The declaration side is built and tested; the runtime side has not started. The
anchors below were current when it was written.

### Next

Stage 1 and stage 2 together, on the route table's crossing and `Core\Router::dispatch`:
- carry `ParamIn::Query` params across with their `CaptureConv` (`crates/nvs-cli/src/main.rs:2899`, `crates/nvs-runtime/src/routes.rs:339`)
- read the query once through `crate::uri::parse_query` (`crates/nvs-stdlib/src/request.rs:1773`, `crates/nvs-stdlib/src/uri.rs:3183`)
- fill each parameter in the handler call, which today reads only the captures (`crates/nvs-stdlib/src/router.rs:2634`)
- a bad or missing value refuses like `BAD_CAPTURE` (`crates/nvs-stdlib/src/router.rs:2389`, `crates/nvs-stdlib/src/router.rs:2513`)

### Backlog

- Stage 3's proofs follow in their own session. `docs/rules/routing/a-bad-query-value-is-a-400.md`
