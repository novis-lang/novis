---
looper: 1
title: A compiler diagnostic and a test result are records
summary: "A compiler diagnostic and a test result are producers of the diagnostic record, every rendering of either comes from it, the inout narrowing record is closed, and the checks below pass."
context: [crates/nvs-render/src/lib.rs, docs/rules/errors/record-producers.md, docs/rules/errors/diagnostic-record.md]
stages:
  - id: 1
    title: "a compiler diagnostic is a producer, and its snippet redacts a `secret`"
    status: open
    context: [crates/nvs-diagnostics/src/render.rs, crates/nvs-cli/src/check.rs, crates/nvs-cli/tests/check.rs, crates/nvs-cli/tests/ast.rs, data/gaps/nvs-render/compiler-diagnostic-is-not-a-producer.json, docs/rules/security/redaction-reaches-the-tools-own-renderings.md, docs/rules/ide/check-json-is-the-diagnostic-record-as-a-document.md]
    extra:
      context:
        rules: [errors/record-producers, errors/record-transformations, security/redaction-reaches-the-tools-own-renderings, ide/check-json-is-the-diagnostic-record-as-a-document, ide/ast-json-schema-is-frozen]
  - id: 2
    title: "a `#[Test]` result is a producer"
    status: open
    context: [crates/nvs-cli/src/runner.rs, data/gaps/nvs-render/test-result-is-not-a-producer.json, docs/rules/testing/report-formats.md]
    extra:
      context:
        rules: [errors/record-producers, testing/report-formats]
  - id: 3
    title: "an `inout` argument drops a `readonly` path's narrowing"
    status: open
    context: [crates/nvs-types/src/expr/args.rs, crates/nvs-types/src/locals.rs, crates/nvs-ir/src/lower/expr.rs, data/gaps/nvs-ir/an-inout-argument-keeps-a-readonly-path-narrowed.json]
    extra:
      context:
        rules: [types/narrowing, statements/inout-is-the-by-reference-spelling]
checks:
  - id: owners-closes
    stage: 3
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, diagnostic-records]
    reach: true
    floor: false
---

# A compiler diagnostic and a test result are records

## Why

`rule:errors/record-producers` names five producers of the one diagnostic record, and three are
built: `Core\Log::write`, `Core\Debug::dump` and an uncaught `Throwable`. The other two print on
their own. `nvs check` renders a diagnostic through `crates/nvs-diagnostics/src/render.rs` for text
and through `crates/nvs-cli/src/check.rs` for `--json`, so the two renderings are two
implementations. The test runner writes its JSON and JUnit reports by hand in
`crates/nvs-cli/src/runner.rs`. Because a compiler diagnostic is not a producer, its source snippet
prints a `secret` literal in the clear, which is the one surface
`rule:security/redaction-reaches-the-tools-own-renderings` still names. This goal makes both
producers, and closes one checker record an audit found already fixed.

## Standing decisions

**A record closes when its rule ships.** The planning session's call. Each stage deletes its gap
record in the same commit, flips the rule it completes to `shipped`, and rewrites the rule's
"Partly on disk" paragraph as present tense. `bun nv owners --closes diagnostic-records` refuses
the goal while a record or a designed rule names it.

**The `check --json` document is frozen byte for byte, beside the comparison tests.** The planning
session's call. `rule:ide/check-json-is-the-diagnostic-record-as-a-document` says the schema is
frozen and snapshot-tested the way `nvs ast --json`'s is. The tests in
`crates/nvs-cli/tests/check.rs` compare the document with the text rendering on purpose, and they
stay: the snapshot is added beside them, not instead of them.

**Trade-offs.** No program observes a change, and nothing runs on the request path. A diagnostic
builds one record before it renders, which is one allocation per diagnostic on the compile path,
paid only when there is a diagnostic. A test result builds one record per assertion inside the test
runner. Simplicity of the implementation gains: each output format renders from one record, and
redaction lives in one place.

## Stages

### 1. a compiler diagnostic is a producer, and its snippet redacts a `secret`

**Does:** a `Diagnostic` becomes the record's Span nodes over a source map, the terminal text and
the `--json` document both render from that record, and a `secret` literal in a snippet prints as
the redaction node.

- [ ] A compiler diagnostic builds the record (`rule:errors/record-producers`), and the terminal renderer renders from it. `crates/nvs-diagnostics/src/render.rs:107`, `crates/nvs-render/src/lib.rs:1`
- [ ] `nvs check --json` renders the same record, so `diagnostic_json` stops building its own value (`rule:ide/check-json-is-the-diagnostic-record-as-a-document`). `crates/nvs-cli/src/check.rs:70`, `crates/nvs-cli/src/check.rs:292`
- [ ] A `secret` literal inside a snippet's lines prints redacted in the text and the JSON, with a test (`rule:security/redaction-reaches-the-tools-own-renderings`). `crates/nvs-diagnostics/src/render.rs:107`
- [ ] One example's whole `--json` document is frozen byte for byte, as `crates/nvs-cli/tests/ast.rs:178` freezes `ast --json`, and the module doc of `crates/nvs-cli/tests/check.rs:6` says why both kinds of test exist.
- [ ] The gap record is deleted, and `rule:security/redaction-reaches-the-tools-own-renderings` is `shipped`.

### 2. a `#[Test]` result is a producer

**Does:** a test result is a record per assertion, with expected and actual as sibling nodes, and
the runner's JSON and JUnit reports render from it.

- [ ] A test result builds the record (`rule:errors/record-producers`). `crates/nvs-cli/src/runner.rs:2647`
- [ ] The JSON report and the JUnit document render from the record, and their output stays byte for byte what `rule:testing/report-formats` pins. `crates/nvs-cli/src/runner.rs:2647`, `crates/nvs-cli/src/runner.rs:2826`
- [ ] The gap record is deleted, and `rule:errors/record-producers` is `shipped`, its "Partly on disk" paragraph rewritten.

### 3. an `inout` argument drops a `readonly` path's narrowing

**Does:** the record's claim is settled by a test. An audit found the checker already counts an
`inout` argument as a write of its path.

- [ ] A test narrows a `readonly` property, passes it as an `inout` argument, and reads it again: the checker treats the read as the declared type (`rule:types/narrowing`). `crates/nvs-types/src/expr/args.rs:180`, `crates/nvs-types/src/locals.rs:956`
- [ ] If the test passes, the record is deleted. If it fails, `note_write` covers the path, and then the record is deleted. `data/gaps/nvs-ir/an-inout-argument-keeps-a-readonly-path-narrowed.json:1`
- [ ] `bun nv owners --closes diagnostic-records` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written.

### Next

Stage 1, the compiler diagnostic, on `crates/nvs-diagnostics/src/render.rs`,
`crates/nvs-cli/src/check.rs` and `crates/nvs-render/src/lib.rs`:
- the record built from a `Diagnostic`, and the text renderer reading it (`rule:errors/record-producers`). `crates/nvs-diagnostics/src/render.rs:107`
- `check --json` from the same record (`rule:ide/check-json-is-the-diagnostic-record-as-a-document`). `crates/nvs-cli/src/check.rs:292`
- the `secret` snippet redaction (`rule:security/redaction-reaches-the-tools-own-renderings`). `crates/nvs-diagnostics/src/render.rs:107`

### Backlog

- A published JSON schema for `check --json` is not planned: `nvs ast --json` publishes none either (`docs/rules/ide/ast-json-schema-is-frozen.md`).
