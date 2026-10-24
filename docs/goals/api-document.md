---
looper: 1
title: The generated API document passes a strict OpenAPI validator
summary: "A security scheme is declared in code and checked, a class response and a request body have real schemas, a strict OpenAPI 3.1 validator accepts a fixture's document, every gap record this goal owns is closed, and the checks below pass."
context: [crates/nvs-cli/src/openapi.rs, crates/nvs-types/src/routes.rs, crates/nvs-cli/tests/openapi.rs]
stages:
  - id: 1
    title: a security scheme is declared in code and checked
    status: done
    context: [crates/nvs-types/src/routes.rs, crates/nvs-types/src/derive.rs, crates/nvs-cli/src/openapi.rs, data/gaps/nvs-cli/components-securityschemes.json, docs/rules/routing/a-security-scheme-is-declared-in-code.md, docs/rules/attributes/api-adds-and-cannot-contradict.md, docs/rules/core-api/no-mode-strings.md]
    extra:
      context:
        rules: [routing/a-security-scheme-is-declared-in-code, attributes/api-adds-and-cannot-contradict, routing/api-document-is-generated-from-the-route-table, core-api/no-mode-strings]
  - id: 5
    title: the security scheme joins its feature's proofs
    status: done
    context: [docs/examples/lang/attributes/core-api-and-the-openapi-document/about.md, docs/examples/lang/attributes/core-api-and-the-openapi-document/02-name-the-errors-and-the-security-scheme.nvs, crates/nvs-types/src/derive.rs, docs/rules/routing/a-security-scheme-is-declared-in-code.md]
    extra:
      context:
        rules: [routing/a-security-scheme-is-declared-in-code, testing/feature-proofs]
  - id: 2
    title: a response that is a class has its codec's schema
    status: done
    context: [crates/nvs-cli/src/openapi.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/expr_table.rs, data/gaps/nvs-cli/response-body-that-is-an-object.json, data/gaps/nvs-cli/a-response-schema-for-a-nullable.json, docs/rules/core-classes/derive-attribute.md]
    extra:
      context:
        rules: [routing/api-document-is-generated-from-the-route-table, core-classes/derive-attribute, attributes/api-adds-and-cannot-contradict]
  - id: 3
    title: a request body is read from the handler's call, and the document describes it
    status: done
    context: [crates/nvs-cli/src/openapi.rs, crates/nvs-types/src/routes.rs, crates/nvs-types/src/check.rs, data/gaps/nvs-cli/request-body-schemas.json, docs/rules/routing/a-request-body-is-read-from-the-handlers-own-call.md]
    extra:
      context:
        rules: [routing/a-request-body-is-read-from-the-handlers-own-call, routing/api-document-is-generated-from-the-route-table, attributes/api-adds-and-cannot-contradict, core-classes/derive-attribute]
  - id: 6
    title: "`nvs check` warns at a route whose body is read in a helper"
    status: open
    context: [crates/nvs-types/src/routes.rs, crates/nvs-diagnostics/src/code/warnings.rs, docs/rules/routing/a-request-body-is-read-from-the-handlers-own-call.md]
    extra:
      context:
        rules: [routing/a-request-body-is-read-from-the-handlers-own-call]
  - id: 4
    title: a strict validator accepts the document, and the editor completes `#[Api]`
    status: open
    context: [crates/nvs-cli/tests/openapi.rs, crates/nvs-cli/src/api_diff.rs, crates/nvs-lsp/src/completion.rs, crates/nvs-types/src/derive.rs]
    extra:
      context:
        rules: [routing/api-document-is-generated-from-the-route-table, routing/api-diff-fails-a-breaking-change, ide/completion-offers-only-what-the-compiler-derived]
  - id: 7
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, api-document]
    reach: true
    floor: false
---

# The generated API document passes a strict OpenAPI validator

## Why

`nvs build --openapi` writes an OpenAPI 3.1 document that a strict validator refuses. An operation's
`security` names schemes that the document never defines, because nothing in the tree declares one
(`crates/nvs-cli/src/openapi.rs:155`). A handler that returns a class gets the empty schema, because
the codec's field roster is not on the route row (`crates/nvs-cli/src/openapi.rs:206`), and a
`?T`, `array<T>` or enum return gets the empty schema too (`crates/nvs-cli/src/openapi.rs:355`). No
operation has a `requestBody` at all, because the row does not record the type a handler decodes its
body into (`crates/nvs-types/src/routes.rs:324`). This goal closes the four gap records, then proves
the result with a strict validator in a test. `nvs api diff` already compares `responses` and
`requestBody`, so a complete document is what makes that gate useful.

## Standing decisions

**A security scheme is declared in code.** The user decided this. A compiler-recognized
`#[Api\Scheme]` on a class names each scheme, the compiler checks every `#[Api(security: [...])]` name
against the declared set, and the route table carries the schemes to `components.securitySchemes`.
`rule:routing/a-security-scheme-is-declared-in-code` holds it. Stage 1 fixes the field table in the rule
before the code, following `#[Api]`'s conventions. The scheme's `type` is an enum, `Core\Api\SchemeType`,
never a string (`rule:core-api/no-mode-strings`). Trade-offs: a new compile error for a typo, one new
attribute on the closed roster, and one table row per scheme while compiling. Nothing runs per request.

**A class response is read from the codec table the front end already built.** The emitter reads the
returned class's `#[Json\Derive]` roster through `ExprTable::codec`
(`crates/nvs-types/src/expr_table.rs:1860`), and `nvs build --openapi` passes the checked program's
table to `openapi::document` (`crates/nvs-cli/src/main.rs:1239`). That table is in scope at the call,
so the row carries no roster, and a compile that writes no document pays for nothing new. Only the
roster is published, never a class's private properties
(`data/gaps/nvs-cli/response-body-that-is-an-object.json`).

**A request body is read from the handler's own call.** The user decided this. The compiler records a
`Core\Request::jsonAs<T>()` or `postAs<T>()` call written directly in a route handler, and the route row
carries "body: `T`, JSON or form". There is no new syntax, so the document cannot disagree with the
code. A body read inside a helper method is not seen: the document then has no `requestBody`, and `nvs
check` warns at the route. `rule:routing/a-request-body-is-read-from-the-handlers-own-call` holds it.
The user decided that the warning follows resolved calls through any number of methods and not
through a `callable` or `mixed` receiver. Two reads of one kind give a `oneOf`, and required-ness
follows what `jsonAs` and `postAs` throw. Trade-offs: one body entry per route row while compiling,
and a walk over resolved calls in `nvs check` alone. Nothing runs per request. A handler author who
reads the body in a helper gets a warning and moves the read.

**The validator is the `jsonschema` crate as a dev-dependency of `nvs-cli`, over the official
OpenAPI 3.1 schema.** `jsonschema` with `default-features = false` (no network resolver), and the
3.1 schema and its `schema-base` dialect vendored as fixtures. The test also walks the document and
asserts that every `$ref` and every `security` name resolves, which the schema alone does not check.
Why: the test stays in `cargo test` beside the other document tests, needs no network and no npm
tool calling the binary, and no build of `nvs` links it. The crate is MIT, which `deny.toml` allows.

**Lanes.** Stages 1, 2 and 3 share `openapi.rs` and `routes.rs`, so they run in order 1, 2, 3.
Stage 5 (the proofs under `docs/examples/`) needs stage 1 only, and runs any time after it. Stage 6
(the warning, `routes.rs` and `warnings.rs`) runs after 3. Stage 4's completion item
(`crates/nvs-lsp/src/completion.rs`) and its `api_diff.rs` doc rewrite are disjoint from `nvs-cli`'s
emitter and can run after stage 1. Its validator test runs last, after 2 and 3.

**Trade-offs, all together.** Request path: nothing. Compile path: one row per scheme and one body
entry per route. Usability: a typo in `security` becomes a compile error, and a generated client gets
real types for a class response and a body. Simplicity: one new attribute, one new warning, and no new
way to declare a body. A dev-dependency adds to the lock file and to no binary.

## Stages

### 1. a security scheme is declared in code and checked

**Does:** `#[Api\Scheme]` declares a scheme, `security` names are checked against the declarations,
and the document writes `components.securitySchemes`.

- [x] The rule's field table is written before the code: each OpenAPI scheme type, its required fields, and `type` as the enum `Core\Api\SchemeType` (`rule:core-api/no-mode-strings`). `docs/rules/routing/a-security-scheme-is-declared-in-code.md:1`
- [x] `Core\Api\Scheme` joins the recognized roster with its `ATTRIBUTE_DOCS` card, and the checker collects every declaration into the route table. `crates/nvs-types/src/derive.rs:593`, `crates/nvs-types/src/derive.rs:191`, `crates/nvs-types/src/routes.rs:625`
- [x] A `security` entry naming no declared scheme, a name declared twice, and a field the type does not take are compile errors, each with a code and a test (`rule:routing/a-security-scheme-is-declared-in-code`). `crates/nvs-types/src/routes.rs:1092`, `crates/nvs-types/src/routes.rs:425`
- [x] The document writes `components.securitySchemes`, sorted by name. `crates/nvs-cli/src/openapi.rs:47`, `crates/nvs-cli/src/openapi.rs:155`
- [x] The gap record deleted. `data/gaps/nvs-cli/components-securityschemes.json:1`

### 5. the security scheme joins its feature's proofs

**Does:** the scheme is in the feature's example, `about.md` and help card, and the rule ships.

**Split:** one writing subagent updates the example and `about.md` together, held to `AGENTS.md`
§ *Text an end user reads*.

- [x] Example 02 declares the scheme it names, and its `.out` is regenerated. `docs/examples/lang/attributes/core-api-and-the-openapi-document/02-name-the-errors-and-the-security-scheme.nvs:1`
- [x] `about.md` says how a scheme is declared. `docs/examples/lang/attributes/core-api-and-the-openapi-document/about.md:1`
- [x] The help card of `Core\Api\Scheme` in the binary. `crates/nvs-types/src/derive.rs:191`
- [x] `bun nv proofs --id 'lang:attributes/core-api-and-the-openapi-document'` passes (`rule:testing/feature-proofs`). `docs/examples/lang/attributes/core-api-and-the-openapi-document/about.md:1`
- [x] The rule `shipped`. `data/rules/routing/a-security-scheme-is-declared-in-code.json:3`

### 2. a response that is a class has its codec's schema

**Does:** a handler that returns a `#[Json\Derive]` class gets an object schema with the codec's
fields, and so does an `errors` entry's class. A `?T`, `array<T>` or enum return gets its real
schema.

- [x] `openapi::document` takes the checked program's expression table, and the emitter reads the class's `DerivedCodec` from it. `crates/nvs-cli/src/main.rs:1239`, `crates/nvs-types/src/expr_table.rs:1860`, `crates/nvs-types/src/derive.rs:749`
- [x] A class response renders as `components.schemas` entries referenced by `$ref`, with each field's type and required-ness, and a nested class is its own entry. `crates/nvs-cli/src/openapi.rs:197`
- [x] An `errors` entry's class gets the same schema under its status. `crates/nvs-cli/src/openapi.rs:222`
- [x] A `?T` return renders `T` with `null` allowed, an `array<T>` renders `type: array` with `T` as `items`, and an enum lists its cases. `crates/nvs-cli/src/openapi.rs:355`
- [x] Tests over a class with a skipped field and a private property, and one each for `?T`, `array<T>` and an enum. `crates/nvs-cli/tests/openapi.rs:50`
- [x] Both gap records deleted.

### 3. a request body is read from the handler's call, and the document describes it

**Does:** the row records the type a handler decodes its body into, and the document writes
`requestBody`.

- [x] The checker records each `Core\Request::jsonAs<T>()` and `postAs<T>()` written directly in a route handler, beside the decode sites it already gathers, and the row carries `T` with JSON or form (`rule:routing/a-request-body-is-read-from-the-handlers-own-call`). `crates/nvs-types/src/check.rs:183`, `crates/nvs-types/src/routes.rs:324`
- [x] The document writes `requestBody` with `T`'s schema from stage 2 under its media type, a `oneOf` for two types of one kind, and `required` as the rule says. `crates/nvs-cli/src/openapi.rs:127`
- [x] Tests for a JSON body, a form body, and both. `crates/nvs-cli/tests/openapi.rs:50`
- [x] The gap record deleted. `data/gaps/nvs-cli/request-body-schemas.json:1`

### 6. `nvs check` warns at a route whose body is read in a helper

**Does:** a route whose handler reaches a body read only through resolved calls gets a warning.

- [ ] The walk over resolved calls from the handler, stopping at a `callable` or `mixed` receiver. `crates/nvs-types/src/routes.rs:1092`
- [ ] A new warning code after `W1023`, with its card. `crates/nvs-diagnostics/src/code/warnings.rs:234`
- [ ] Tests: a read in a helper warns, a read two calls deep warns, a read through a `callable` does not. `crates/nvs-types/src/routes.rs:1092`
- [ ] The rule `shipped`. `data/rules/routing/a-request-body-is-read-from-the-handlers-own-call.json:3`

### 4. a strict validator accepts the document, and the editor completes `#[Api]`

**Does:** a test proves the whole document valid, and the editor offers what `#[Api]` admits.

**Split:** one writing subagent builds the fixture program after stages 1 to 3 land: captures,
`#[Query]`, `#[Api]` with all its fields, a scheme of each type, a class response, a `?T` and an
enum response, and a body.

- [ ] The fixture program that uses every feature the document renders. `crates/nvs-cli/tests/fixtures/openapi/every-feature.nvs:1`
- [ ] `jsonschema` as a dev-dependency, and the OpenAPI 3.1 schema and its dialect vendored. `crates/nvs-cli/Cargo.toml:1`, `crates/nvs-cli/tests/fixtures/openapi/schema-3.1.json:1`
- [ ] A test validates the fixture's document and checks that every `$ref` and `security` name resolves. `crates/nvs-cli/tests/openapi.rs:28`
- [ ] Completion inside `#[Api(...)]` offers its fields from the attribute's card, and a `security` entry offers the declared schemes (`rule:ide/completion-offers-only-what-the-compiler-derived`). `crates/nvs-types/src/derive.rs:186`, `crates/nvs-lsp/src/completion.rs:2087`
- [ ] `.lspt` cases for both completions. `tests/lsp/completion/an-api-attribute-offers-its-fields.lspt:1`
- [ ] `nvs api diff`'s module doc is rewritten: the document now carries `responses` and `requestBody`. `crates/nvs-cli/src/api_diff.rs:33`
- [ ] `bun nv owners --closes api-document` passes. `data/rules/routing/a-security-scheme-is-declared-in-code.json:3`

### 7. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`
- [ ] The perf figures this goal made stale are recorded again: `Core\Router` (`router.rs`) and the `lang:attributes` sections (`90-attributes.md`). Otherwise the release gate's `proofs` job is red. `bun nv proofs --record-perf --group 'Core\Router'`, `docs/reference/lang/90-attributes.md:1369`

## Handoff

### State

Stages 1, 5, 2 and 3 are done. The `about.md` is under 200 words, and `routes.mdx` is restamped.

`crate::routes::note_body_read` (`crates/nvs-types/src/routes.rs:2346`) runs in `infer_static_call`
beside `written_class_of`. It records `Core\Request::jsonAs<T>()` and `postAs<T>()` only in an armed
`BodyWriters`: a `#[Route]` method's own body, its anonymous functions included. `check_method`'s
`keep_body_reads` hands the reads to `RouteTable::note_bodies`, and `check_class_routes` puts them
on `Route::bodies` (`routes.rs:479`) as `RequestBody {kind, ty: BodyTy}`. A `postAs` with an argument
reads one form field and is not recorded. The emitter's `request_body` writes one media type per kind,
a `oneOf` for two schemas, and `required` for JSON or a form class with a required field. Four tests
over `crates/nvs-cli/tests/fixtures/api/request-body.nvs` pass.

The acceptance findings were fixed on HEAD already: the nightly run tested `3ea6f1550`.

A qualified name is absolute, so `use Core\Api;` does not place `Api\Scheme`. Write
`#[Core\Api\Scheme]` or `use Core\Api\Scheme;`.

Tests that may go red at the gate: `nvs-lsp`'s stubs tests (`every_line_lands_on_its_name`, and the
test that every name has a line) for the new `Core\Api\*` names and card, any `nvs-stdlib` test
that counts `ENUMS` or checks enum cards, and any `nvs api diff` test whose fixture returns a
class with a codec, because its document now has `components.schemas`. Any `nvs api diff` test
whose fixture's handler calls `jsonAs` or `postAs`, because its document now has `requestBody`.

### Next

**Stage 6: `nvs check` warns at a route whose body is read in a helper**, as its section says.

- [ ] Record, per method label, the resolved callees its body calls (not through a `callable` or `mixed` receiver), and whether its body has a whole-body read. `note_body_read` returns early for an unarmed body today and needs a second, unarmed record. `crates/nvs-types/src/routes.rs:2346`, `crates/nvs-types/src/check.rs:919`
- [ ] After the walk, for each row with empty `bodies`, walk the callees from its handler and warn at the `#[Route]` when one reaches a read. A handler with a `postAs` that writes `name` reads in its own body and does not warn. `crates/nvs-types/src/routes.rs:913`
- [ ] A new warning code after `W1023`, with its card, and the tests the section names. `crates/nvs-diagnostics/src/code/warnings.rs:234`

The perf figures for `Core\Router` and the `lang:attributes` sections are stale. Stage 7 records them
once, after the last edit to `router.rs` and `90-attributes.md`.

### Backlog

- OAuth2 scopes per operation: `#[Api(security:)]` writes an empty scope list today (`crates/nvs-cli/src/openapi.rs:242`). Scopes need a field on `#[Api]` and a check against the scheme's declared flows (`docs/rules/routing/a-security-scheme-is-declared-in-code.md`).
