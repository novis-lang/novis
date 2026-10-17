---
looper: 1
title: The generated API document passes a strict OpenAPI validator
summary: "A security scheme is declared in code and checked, a class response and a request body have real schemas, a strict OpenAPI 3.1 validator accepts a fixture's document, every gap record this goal owns is closed, and the checks below pass."
context: [crates/nvs-cli/src/openapi.rs, crates/nvs-types/src/routes.rs, crates/nvs-cli/tests/openapi.rs]
stages:
  - id: 1
    title: a security scheme is declared in code and checked
    status: open
    context: [crates/nvs-types/src/routes.rs, crates/nvs-types/src/derive.rs, crates/nvs-cli/src/openapi.rs, data/gaps/nvs-cli/components-securityschemes.json, docs/rules/routing/a-security-scheme-is-declared-in-code.md, docs/rules/attributes/api-adds-and-cannot-contradict.md]
    extra:
      context:
        rules: [routing/a-security-scheme-is-declared-in-code, attributes/api-adds-and-cannot-contradict, routing/api-document-is-generated-from-the-route-table]
  - id: 2
    title: a response that is a class has its codec's schema
    status: open
    context: [crates/nvs-cli/src/openapi.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/routes.rs, data/gaps/nvs-cli/response-body-that-is-an-object.json, docs/rules/core-classes/derive-attribute.md]
    extra:
      context:
        rules: [routing/api-document-is-generated-from-the-route-table, core-classes/derive-attribute, attributes/api-adds-and-cannot-contradict]
  - id: 3
    title: a request body is read from the handler's call, and the document describes it
    status: open
    context: [crates/nvs-cli/src/openapi.rs, crates/nvs-types/src/routes.rs, crates/nvs-types/src/expr/args.rs, data/gaps/nvs-cli/request-body-schemas.json, docs/rules/routing/a-request-body-is-read-from-the-handlers-own-call.md]
    extra:
      context:
        rules: [routing/a-request-body-is-read-from-the-handlers-own-call, routing/api-document-is-generated-from-the-route-table, attributes/api-adds-and-cannot-contradict, core-classes/derive-attribute]
  - id: 4
    title: a strict validator accepts the document, and the editor completes `#[Api]`
    status: open
    context: [crates/nvs-cli/tests/openapi.rs, crates/nvs-cli/src/api_diff.rs, crates/nvs-lsp/src/completion.rs, crates/nvs-types/src/derive.rs]
    extra:
      context:
        rules: [routing/api-document-is-generated-from-the-route-table, routing/api-diff-fails-a-breaking-change, ide/completion-offers-only-what-the-compiler-derived]
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
(`crates/nvs-cli/src/openapi.rs:155-166`). A handler that returns a class gets the empty schema, because
the codec's field roster is not on the route row (`crates/nvs-cli/src/openapi.rs:196-205`). No operation
has a `requestBody` at all, because the row does not record the type a handler decodes its body into
(`crates/nvs-types/src/routes.rs:346-356`). This goal closes the three gap
records, then proves the result with a strict validator in a test. `nvs api diff` already compares
`responses` and `requestBody`, so a complete document is what makes that gate useful.

## Standing decisions

**A security scheme is declared in code.** The user decided this. A compiler-recognized
`#[Api\Scheme]` on a class names each scheme, the compiler checks every `#[Api(security: [...])]` name
against the declared set, and the route table carries the schemes to `components.securitySchemes`.
`rule:routing/a-security-scheme-is-declared-in-code` holds it. Stage 1 fixes the field table in the rule
before the code, following `#[Api]`'s conventions. Trade-offs: a new compile error for a typo, one new
attribute on the closed roster, and one table row per scheme while compiling. Nothing runs per request.

**A class response is read from the codec table the front end already built.** The planning
session's call. The row names the returned class, and the emitter reads that class's
`#[Json\Derive]` roster (`crates/nvs-types/src/derive.rs:750`) from the checked program. The roster is
not copied onto every row, so a compile that writes no document pays for nothing new. If the emitter
cannot reach the codec table without resolving anything itself, the row carries the roster instead, and
stage 2 states what that costs per route. Only the roster is published, never a class's private
properties (`data/gaps/nvs-cli/response-body-that-is-an-object.json`).

**A request body is read from the handler's own call.** The user decided this. The compiler records a
`Core\Request::jsonAs<T>()` or `postAs<T>()` call written directly in a route handler, and the route row
carries "body: `T`, JSON or form". There is no new syntax, so the document cannot disagree with the
code. A body read inside a helper method is not seen: the document then has no `requestBody`, and `nvs
check` warns at the route. `rule:routing/a-request-body-is-read-from-the-handlers-own-call` holds it.
The user decided that the warning follows resolved calls through any number of methods and not
through a `callable` or `mixed` receiver. The planning session's calls in that rule: two reads of one
kind give a `oneOf`, and required-ness follows what `jsonAs` and `postAs` throw. Trade-offs: one body entry per route row while
compiling, and a walk over resolved calls in `nvs check` alone. Nothing runs per request. A handler
author who reads the body in a helper gets a warning and moves the read.

**The validator is a test dependency only.** The planning session's call. The stage picks a maintained
validator that implements OpenAPI 3.1 and its JSON Schema dialect, as a dev-dependency (a caret range in
`package.json`, or a Rust `dev-dependencies` entry). No build of `nvs` links it.

**Trade-offs, all together.** Request path: nothing. Compile path: one row per scheme and one body
entry per route. Usability: a typo in `security` becomes a compile error, and a generated client gets
real types for a class response and a body. Simplicity: one new attribute, one new warning, and no new
way to declare a body.

## Stages

### 1. a security scheme is declared in code and checked

**Does:** `#[Api\Scheme]` declares a scheme, `security` names are checked against the declarations,
and the document writes `components.securitySchemes`.

- [ ] The rule's field table is written before the code: each OpenAPI scheme type, its required fields, and whether `type` is a string or an enum (`rule:core-api/no-mode-strings`). `docs/rules/routing/a-security-scheme-is-declared-in-code.md`
- [ ] `Core\Api\Scheme` joins the recognized roster with its `ATTRIBUTE_DOCS` card, and the checker collects every declaration into the route table. `crates/nvs-types/src/derive.rs:594`, `crates/nvs-types/src/derive.rs:191`, `crates/nvs-types/src/routes.rs:324`
- [ ] A `security` entry naming no declared scheme, a name declared twice, and a field the type does not take are compile errors, each with a code and a test (`rule:routing/a-security-scheme-is-declared-in-code`). `crates/nvs-types/src/routes.rs:1097`
- [ ] The document writes `components.securitySchemes`, sorted by name, and `data/gaps/nvs-cli/components-securityschemes.json` is deleted. `crates/nvs-cli/src/openapi.rs:75-82`, `crates/nvs-cli/src/openapi.rs:149-154`
- [ ] The scheme joins the feature's proofs: an example, and the help card. `bun nv proofs --id 'lang:attributes/core-api-and-the-openapi-document'` passes. The rule is `shipped`.

### 2. a response that is a class has its codec's schema

**Does:** a handler that returns a `#[Json\Derive]` class gets an object schema with the codec's
fields, and so does an `errors` entry's class.

- [ ] The emitter reaches the class's `DerivedCodec`, or the row carries it, as the standing decision says. `crates/nvs-types/src/derive.rs:750`, `crates/nvs-cli/src/openapi.rs:354`
- [ ] A class response renders as `components.schemas` entries referenced by `$ref`, with each field's type and required-ness, and a nested class is its own entry. `crates/nvs-cli/src/openapi.rs:196-205`
- [ ] An `errors` entry's class gets the same schema under its status. `crates/nvs-cli/src/openapi.rs:180-183`
- [ ] Tests over a class with a skipped field and a private property, and `data/gaps/nvs-cli/response-body-that-is-an-object.json` deleted.

### 3. a request body is read from the handler's call, and the document describes it

**Does:** the row records the type a handler decodes its body into, the document writes `requestBody`,
and `nvs check` warns when the read is in a helper.

- [ ] The checker records each `Core\Request::jsonAs<T>()` and `postAs<T>()` written directly in a route handler, and the row carries `T` with JSON or form (`rule:routing/a-request-body-is-read-from-the-handlers-own-call`). `crates/nvs-types/src/expr/args.rs:1530`, `crates/nvs-types/src/routes.rs:356`
- [ ] The document writes `requestBody` with `T`'s schema from stage 2 under its media type, a `oneOf` for two types of one kind, and `required` as the rule says. `crates/nvs-cli/src/openapi.rs:127`
- [ ] `nvs check` warns at a route whose handler reaches a body read only through resolved calls, with a new warning code, its card and a test. `crates/nvs-diagnostics/src/lib.rs:5624`
- [ ] Tests for a JSON body, a form body, both, and a read in a helper. `data/gaps/nvs-cli/request-body-schemas.json` deleted, and the rule `shipped`.

### 4. a strict validator accepts the document, and the editor completes `#[Api]`

**Does:** a test proves the whole document valid, and the editor offers what `#[Api]` admits.

- [ ] A fixture program uses every feature the document renders: captures, `#[Query]`, `#[Api]` with all its fields, a scheme of each type, a class response and a body. A test runs the strict validator over its document. `crates/nvs-cli/tests/openapi.rs`
- [ ] Completion inside `#[Api(...)]` offers its fields from the attribute's card, and a `security` entry offers the declared schemes (`rule:ide/completion-offers-only-what-the-compiler-derived`). `crates/nvs-types/src/derive.rs:186`, `crates/nvs-lsp/src/completion.rs:1992`
- [ ] `nvs api diff`'s module doc is rewritten: the document now carries `responses`, and `requestBody` after stage 3. `crates/nvs-cli/src/api_diff.rs:33-37`
- [ ] `bun nv owners --closes api-document` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written.

### Next

Stage 1, the security scheme, on `crates/nvs-types/src/routes.rs`, `crates/nvs-types/src/derive.rs` and
`crates/nvs-cli/src/openapi.rs`:
- write the field table into `rule:routing/a-security-scheme-is-declared-in-code`.
- `Core\Api\Scheme` on the roster, collected into the route table. `crates/nvs-types/src/derive.rs:594`
- the three compile errors. `crates/nvs-types/src/routes.rs:1097`
- `components.securitySchemes`. `crates/nvs-cli/src/openapi.rs:75-82`

### Backlog

- OAuth2 scopes per operation: `#[Api(security:)]` writes an empty scope list today (`crates/nvs-cli/src/openapi.rs:149-154`). Scopes need a field on `#[Api]` and a check against the scheme's declared flows (`docs/rules/routing/a-security-scheme-is-declared-in-code.md`).
