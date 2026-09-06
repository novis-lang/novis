# ADR 0085 — The API document is generated while compiling, so it cannot drift from the code

- **Status:** Accepted
- **Date:** 2026-08-24
- **Scope:** generating an OpenAPI 3.1 document from [0077](0077-compile-time-routing.md)'s route table,
  [0071](0071-derived-codecs.md)'s derived codecs and declared types; the `#[Api]` attribute for what the
  types cannot say; which mismatches are compile errors; and `nvs api diff`, the breaking-change gate. Not
  in scope: generating *clients*, which existing generators consume this document to do
  ([0051](0051-standard-library-tiers.md)'s domain-logic rule), and serving the document, which is
  `Web\Api`'s ([0082](0082-the-first-party-framework.md) § 3).
- **Amends:** [0071](0071-derived-codecs.md) § 1 — `Core\Api` joins the closed, `Core`-owned list of
  compiler-recognized attributes, matched by name exactly as `#[Route]` and `#[Json\Derive]` are.
  [0077](0077-compile-time-routing.md) — its route table gains a second consumer; nothing about matching,
  the pattern grammar or its three compile errors changes.
  [0019](0019-reflection-and-ast-parsing-are-core-features.md) — one more thing its *Consequences* named as
  a userland use of reflection is a compiler pass instead, for the same reason routing became one.
  [docs/implementation-plan.md](../implementation-plan.md) — M4S gains the emitter, M7 the serving.
- **Amended by:** 0102, 0110
- **Depends on:** [0077](0077-compile-time-routing.md) and [0071](0071-derived-codecs.md) — with neither,
  there is nothing to generate from.

> **In short:** Novis already knows every route while compiling ([0077](0077-compile-time-routing.md)) and
> already derives a codec from a class's declared properties ([0071](0071-derived-codecs.md)). An OpenAPI
> 3.1 document is those two facts written out, so it is **generated, not maintained** — and a specification
> that is generated from the code cannot disagree with the code, which is the failure every hand-written or
> annotation-scanned API document eventually has. `nvs build --openapi` writes it; nothing is emitted for a
> program that does not ask. What the types cannot express — a summary, an error response, a security
> scheme, an example — comes from a `#[Api]` attribute and from the declaration's own doc comment, and
> **an `#[Api]` that contradicts the code is a compile error**, not a documentation bug. `nvs api diff`
> compares two generated documents and fails a build on a breaking change, which turns "we broke a client"
> from an incident into a red pipeline.

## Context

- **Every API document in every framework drifts**, because it is a second description of a thing that
  already exists. Hand-written OpenAPI drifts immediately. Annotation-scanned OpenAPI drifts more slowly and
  more confusingly, because the annotation looks authoritative while being unchecked — a `@OA\Response`
  claiming a shape the controller no longer returns is the normal state of a mature PHP or Python codebase.
- **Novis is in the unusual position of already having the facts.** A route's path, method, name and captured
  parameter types are compiler data ([0077](0077-compile-time-routing.md)). A request or response body's
  shape is a class's declared properties, which [0071](0071-derived-codecs.md) already reads — field names,
  declared types, nullability and constructor position — to build a codec. Nothing needs to be discovered;
  it needs to be printed.
- **The type system makes the output unusually precise.** Every binding has a declared type
  ([0007](0007-explicit-type-system.md)), a property is definitely initialized
  ([0022](0022-definite-property-initialization.md)) so required-vs-optional is decidable rather than
  guessed, `?T` is the only nullability ([0066](0066-nullable-conversion-operator.md)), unions and literal
  types are expressible ([0047](0047-literal-and-enum-case-types.md)), enums are closed
  ([0010](0010-enums-are-a-value-type.md)) and a shape type is structural
  ([0036](0036-anonymous-object-shapes.md)). Each maps onto JSON Schema with no loss, which is why 3.1 —
  whose schema dialect *is* JSON Schema — is the target.
- **It is nearly free, and it is a real differentiator.** For any application something else integrates
  against ([0080](0080-the-audience-nvs-is-built-for.md)), "the contract is generated from the
  implementation and CI fails if you break it" is a property teams pay for, and no incumbent can offer it
  without the type system to back it.

## Decision

### 1. What supplies what

| Part of the document | Comes from |
|---|---|
| Path, method, operation id | `#[Route]`'s `path`, `method` and `name` ([0077](0077-compile-time-routing.md)); a `name` two operations share carries the lowercased verb as a suffix, so `operationId` stays unique ([0110](0110-one-methods-repeated-routes-share-a-name-when-they-share-a-path.md) § 3) |
| Path parameters and their schemas | the handler's own parameters, by declared type — the same binding [0077](0077-compile-time-routing.md) § 3 already makes |
| Query parameters | `#[Query]` parameters, by declared type, with a parameter default making one optional ([0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § 3) |
| Request body schema | the `#[Json\Derive]` codec of the body parameter's class ([0071](0071-derived-codecs.md)) |
| Response body schema | the handler's declared return type, through the same codec |
| Required vs optional | definite initialization ([0022](0022-definite-property-initialization.md)) and parameter defaults — decidable, never guessed |
| Nullability | `?T` and nothing else ([0066](0066-nullable-conversion-operator.md)) |
| Enumerations | a closed enum's cases, or a literal-type union ([0010](0010-enums-are-a-value-type.md), [0047](0047-literal-and-enum-case-types.md)) |
| Summary and description | the declaration's own doc comment — first sentence is the summary, remainder the description |
| Error responses, examples, deprecation, tags, security scheme | `#[Api]`, § 2 |

**A program with no `#[Route]` generates nothing and runs no pass**, the same "pay for what you ask for"
rule [0077](0077-compile-time-routing.md) and [0061](0061-compile-time-autoload-and-program-discovery.md)
already hold to.

### 2. `#[Api]` — only what the types cannot say

```php
#[Route(path: "/orders/{id}", method: Http\Method::Get, name: "orders.show")]
#[Access(allow: Role::User)]
#[Api(
    tags:      ["Orders"],
    errors:    [{status: 404, type: Api\NotFound}, {status: 403, type: Api\Forbidden}],
    security:  ["bearer"],
    example:   {id: 7, total: "19.99"},
)]
public static function show(uint $id): Order { … }
```

`#[Api]` is a compiler-recognized attribute in [0071](0071-derived-codecs.md) § 1's closed list, matched by
name rather than structurally, so a userland shape literal that happens to look like one is not one
([0046](0046-attributes-shape-literal-metadata.md) § 4).

**It may add, and it may not contradict.** These are compile errors, each naming both sites:

- an `errors` entry whose `type` is not a class the handler could produce;
- a `security` scheme name that no configured scheme defines;
- an `example` that does not decode against the schema the return type generates — checked with the same
  decoder [0071](0071-derived-codecs.md) already built, so the example is validated by the real codec
  rather than by a second implementation;
- an `#[Api]` on a method with no `#[Route]`.

That list is the whole point of the ADR: the annotation cannot lie, because the compiler holds both halves.

### 3. Emission

`nvs build --openapi <path>` writes OpenAPI 3.1 JSON. It is a build artifact, not a runtime feature: the
running server does not construct it, and `Web\Api` merely serves a file the build produced
([0082](0082-the-first-party-framework.md) § 3). Determinism is required — routes and schemas are emitted in
a stable order — because § 4 diffs the output and because a document that reorders itself between builds
makes every diff useless.

### 4. `nvs api diff` — the gate

`nvs api diff <old.json> <new.json>` classifies every change as **breaking**, **additive** or **cosmetic**,
and exits non-zero on a breaking one. Breaking is the ordinary API-compatibility reading: a removed
operation, a removed or newly required field, a narrowed type, a removed enum case, a changed status code.
Additive is a new optional field, a new operation, a new enum case in a response.

Two consequences worth stating: a team can put it in CI against the document from the last release and stop
shipping breaking changes by accident, and the classification is *mechanical* — derived from schemas the
compiler produced — rather than a reviewer's judgement.

## Consequences

- **The document is only as good as the handlers' types**, which in Novis is very good — but a handler
  returning `mixed` produces a useless schema. That is visible in the output rather than hidden, and it is
  the correct incentive.
- **Doc comments become load-bearing** for summaries. They are already parsed for tooling
  ([0040](0040-vscode-deep-tooling-and-resilient-parsing.md)), so this adds no machinery, but it does mean a
  doc comment is now part of a published artifact.
- **`#[Api]` is a fifth compiler-recognized attribute**, growing a list [0071](0071-derived-codecs.md)
  deliberately keeps closed. It earns the place by being checkable against the code; an attribute the
  compiler could not verify would not.
- **OpenAPI 3.1 only.** No 3.0 downgrade path, no Swagger 2. Emitting a lossy older version would mean
  deciding what to drop when the type system says something 3.0 cannot express, and one output format is
  [0051](0051-standard-library-tiers.md) test 6's rule applied here.
- **Non-JSON APIs get nothing from this.** A handler returning HTML or a stream appears in the document as
  an operation with an opaque response, which is honest and not very useful. GraphQL and gRPC are out of
  scope entirely.
- **`nvs api diff` will occasionally be wrong at the margins** — a change that is technically breaking and
  practically harmless. A suppression mechanism is deliberately not provided in v1; if it proves necessary
  it is an additive change, and providing one too early would make the gate advisory.

## Alternatives rejected

- **Runtime reflection over the route table**, the way annotation-scanning frameworks do it. Rejected for
  [0077](0077-compile-time-routing.md)'s reason: it is compile-time work being done at runtime, it needs a
  cache that goes stale, and — decisively — it cannot make a contradiction a compile error, which is the
  entire value here.
- **A hand-written specification, with the compiler checking the code against it** (contract-first). A
  legitimate design, and better for teams who agree the contract before implementing. Rejected as the
  default because it makes the document a second source of truth that must be kept in sync in the other
  direction, and because the checking would have to be strong enough to be worth it — which is a much larger
  feature. Nothing here prevents a team from diffing a generated document against a hand-written one.
- **Generating clients too.** Rejected under [0051](0051-standard-library-tiers.md)'s domain-logic rule: the
  OpenAPI generator ecosystem exists, is specialised per target language, and is exactly the kind of thing
  Novis should emit input for rather than reimplement.
- **Making the document a runtime endpoint the server builds.** Rejected because it puts work on the request
  path for a value that changes only when the code does, which is what the artifact cache is for.

## Verification

- **Fidelity:** for a fixture application, every route in the table appears exactly once in the document;
  every property of every response class appears with the right type, nullability and required-ness;
  removing a property from a class changes the document and nothing else.
- **Contradictions are compile errors:** an `errors` entry naming an unreachable type, an unknown `security`
  scheme, an `example` that fails the real decoder, and an `#[Api]` without a `#[Route]` each fail with a
  diagnostic naming both sites.
- **Nominal matching:** a userland `#[Api(...)]`-shaped literal produces no entry, asserted the way
  [0077](0077-compile-time-routing.md) already asserts it for `#[Route]`.
- **Determinism:** two builds of the same source produce byte-identical documents, on all three platforms.
- **Zero cost when unused:** a program with no `#[Route]` runs no emitter pass and produces no artifact.
- **The gate:** removing an operation, adding a required field and narrowing a type each exit non-zero from
  `nvs api diff`; adding an optional field and a new operation each exit zero.
- **Qualifiers:** a `secret`-typed property cannot appear in a generated schema — it cannot be serialized at
  all ([0033](0033-secret-qualifier-for-confidential-values.md)) — and a class containing one used as a
  response type is a compile error at the handler, not a silently omitted field.
