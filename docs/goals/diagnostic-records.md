---
looper: 1
title: A compiler diagnostic and a test result are records
summary: "A compiler diagnostic and a test result are producers of the diagnostic record, every rendering of either comes from it, the inout narrowing record is closed, and the checks below pass."
context: [crates/nvs-render/src/lib.rs, docs/rules/errors/record-producers.md, docs/rules/errors/diagnostic-record.md]
stages:
  - id: 1
    title: "a compiler diagnostic builds the record, and its text renders from it"
    status: done
    context: [crates/nvs-diagnostics/src/render.rs, crates/nvs-render/src/text.rs]
    extra:
      context:
        rules: [errors/record-producers, errors/record-transformations]
  - id: 4
    title: "`nvs check --json` renders the same record, and its document is frozen"
    status: open
    context: [crates/nvs-cli/src/check.rs, crates/nvs-cli/tests/check.rs, crates/nvs-cli/tests/ast.rs, crates/nvs-render/src/json.rs]
    extra:
      context:
        rules: [ide/check-json-is-the-diagnostic-record-as-a-document, ide/ast-json-schema-is-frozen]
  - id: 5
    title: "a compiler diagnostic's snippet redacts a `secret`"
    status: open
    context: [crates/nvs-diagnostics/src/render.rs, data/gaps/nvs-render/compiler-diagnostic-is-not-a-producer.json, docs/rules/security/redaction-reaches-the-tools-own-renderings.md]
    extra:
      context:
        rules: [security/redaction-reaches-the-tools-own-renderings]
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
    context: [crates/nvs-types/src/expr/args.rs, crates/nvs-types/src/expr/assign.rs, data/gaps/nvs-ir/an-inout-argument-keeps-a-readonly-path-narrowed.json]
    extra:
      context:
        rules: [types/narrowing, statements/inout-is-the-by-reference-spelling]
checks:
  - id: diagnostics-tests
    stage: 1
    name: the diagnostic renderer's tests pass
    run: [bun, nv, verify, --fast, -p, nvs-diagnostics]
  - id: cli-tests
    stage: 4
    name: the check, ast and runner tests pass
    run: [bun, nv, verify, --fast, -p, nvs-cli]
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

**A record closes when its rule ships.** Each stage deletes its gap record in the same commit, flips
the rule it completes to `shipped` in `data/rules/<topic>/<name>.json`, and rewrites the rule's
"Partly on disk" paragraph as present tense. `bun nv owners --closes diagnostic-records` refuses the
goal while a record or a designed rule names it.

**The terminal text stays byte for byte what it is.** About 340 `tests/conformance/reject/` cases
and every snippet `.err` pin the rendered text, so a change of one byte would re-record all of them
for no gain to a reader. Stage 1 is a refactor: the record is built, and the text renderer prints
from it exactly what it prints now. A difference the existing tests show is a bug in the stage, not
an expectation to re-record. Two changes are intended. Stage 1's record substitutes a control byte
in a snippet's source line, as every record does (`rule:errors/record-transformations`), and only
`tests/conformance/lang/an-esc-byte-in-a-string-literal-does-not-compile.nvst` writes one. Stage 5's
redaction reaches only a snippet holding a `secret` literal.

**The `check --json` document is frozen byte for byte, beside the comparison tests.**
`rule:ide/check-json-is-the-diagnostic-record-as-a-document` says the schema is frozen and
snapshot-tested the way `nvs ast --json`'s is. The tests in `crates/nvs-cli/tests/check.rs` compare
the document with the text rendering, and they stay: the snapshot is added beside them, not instead of
them. The fixture is one program written for the test under `crates/nvs-cli/tests/`, with two
diagnostics, so no other test's change moves it.

**Lanes.** Stage 3 is in `nvs-types` and shares no file with any other stage, so it can run first, in
any order, or as a side goal. Stages 1, 4 and 5 run in that order: 4 renders the record 1 builds, and
5 edits `crates/nvs-diagnostics/src/render.rs` after 1. Stage 2 edits only
`crates/nvs-cli/src/runner.rs` and runs after stage 1, whose record-building API it uses; it can run
before or after 4 and 5. Stage 2 flips `rule:errors/record-producers`, so it is the last of 1, 2, 4
and 5 to finish its rule edits. No stage waits on another goal.

**Trade-offs.** No program observes a change, and nothing runs on the request path. A diagnostic
builds one record before it renders, which is one allocation per diagnostic on the compile path,
paid only when there is a diagnostic. A test result builds one record per assertion inside the test
runner. Simplicity of the implementation gains: each output format renders from one record, and
redaction lives in one place.

## Stages

### 1. a compiler diagnostic builds the record, and its text renders from it

**Does:** a `Diagnostic` becomes the record's Span nodes over a source map, and the terminal text
renders from that record, byte for byte as it does now.

**Split:** at stage start, an `Explore` subagent lists every caller of `Renderer::render` and
`render_all` outside `nvs-diagnostics`, as `file.rs:NN` and one line each.

- [x] A compiler diagnostic builds the record (`rule:errors/record-producers`). `crates/nvs-diagnostics/src/render.rs:107`, `crates/nvs-render/src/lib.rs:96`
- [x] The terminal renderer prints from the record, and every existing rendering test passes unchanged. `crates/nvs-diagnostics/src/render.rs:269`, `crates/nvs-diagnostics/src/render.rs:383`

### 4. `nvs check --json` renders the same record, and its document is frozen

**Does:** `nvs check --json` renders the record stage 1 builds, so `diagnostic_json` stops building
its own value, and one whole document is frozen.

- [ ] `nvs check --json` renders the record (`rule:ide/check-json-is-the-diagnostic-record-as-a-document`). `crates/nvs-cli/src/check.rs:158`, `crates/nvs-cli/src/check.rs:372`
- [ ] One fixture's whole `--json` document is frozen byte for byte, as `ast_json_matches_its_frozen_schema` freezes `ast --json` (`rule:ide/ast-json-schema-is-frozen`). `crates/nvs-cli/tests/check.rs:1`, `crates/nvs-cli/tests/ast.rs:184`
- [ ] The module doc of `crates/nvs-cli/tests/check.rs` says why both kinds of test exist. `crates/nvs-cli/tests/check.rs:6`

### 5. a compiler diagnostic's snippet redacts a `secret`

**Does:** a `secret` literal in a snippet's lines prints as the redaction node, in the text and in the
JSON.

- [ ] A `secret` literal inside a snippet's lines prints redacted in the text and the JSON, with a test of each (`rule:security/redaction-reaches-the-tools-own-renderings`). `crates/nvs-diagnostics/src/render.rs:107`, `crates/nvs-cli/tests/check.rs:1`
- [ ] The gap record is deleted. `data/gaps/nvs-render/compiler-diagnostic-is-not-a-producer.json:1`
- [ ] `rule:security/redaction-reaches-the-tools-own-renderings` is `shipped`, its "Partly on disk" paragraph rewritten. `data/rules/security/redaction-reaches-the-tools-own-renderings.json:3`, `docs/rules/security/redaction-reaches-the-tools-own-renderings.md:23`

### 2. a `#[Test]` result is a producer

**Does:** a test result is a record per assertion, with expected and actual as sibling nodes, and
the runner's JSON and JUnit reports render from it.

- [ ] A test result builds the record (`rule:errors/record-producers`). `crates/nvs-cli/src/runner.rs:2647`
- [ ] The JSON report and the JUnit document render from the record, and their output stays byte for byte what `rule:testing/report-formats` pins. `crates/nvs-cli/src/runner.rs:2835`, `crates/nvs-cli/src/runner.rs:3014`
- [ ] The gap record is deleted. `data/gaps/nvs-render/test-result-is-not-a-producer.json:1`
- [ ] `rule:errors/record-producers` is `shipped`, its "Partly on disk" paragraph rewritten. `data/rules/errors/record-producers.json:3`, `docs/rules/errors/record-producers.md:28`

### 3. an `inout` argument drops a `readonly` path's narrowing

**Does:** the record's claim is settled by a test. An audit found the checker already counts an
`inout` argument as a write of its path.

- [ ] A conformance case narrows a `readonly` property, passes it as an `inout` argument, and reads it again: the checker treats the read as the declared type (`rule:types/narrowing`). `tests/conformance/class/an-inout-argument-drops-a-readonly-propertys-narrowing.nvst:1`, `crates/nvs-types/src/expr/args.rs:181`
- [ ] If the case fails, `note_write` covers the path. `crates/nvs-types/src/expr/assign.rs:1714`
- [ ] The record is deleted. `data/gaps/nvs-ir/an-inout-argument-keeps-a-readonly-path-narrowed.json:1`

## Handoff

### State

Stage 1 is done. `nvs_diagnostics::record::record` builds a diagnostic's record, and
`Renderer::render` and `render_all` print the terminal text from it through `render_record`. The
record's shape is the module doc of `crates/nvs-diagnostics/src/record.rs`: the severity and code
are string fields, each label is a `Node::Span` carrying its first line's text, and the notes and
suggestions are sequence fields. `Node::Span` gained a column, an end, `primary` and `source`, and the
JSON rendering writes them as `$span`'s keys. A label in a file the map does not hold is left out of
the record, as the text always left it out. The one byte the text changed is a control byte in a
snippet's source line, which now prints as its control picture (`␛` for `ESC`). The E0011 case that
writes one was re-recorded, because a compiler printing a raw `ESC` to the terminal is the injection
E0011 exists to stop. `nvs check --json` writes such a label with `null`
positions, so stage 4 decides whether the record keeps it.

### Next

**Stage 4: `nvs check --json` renders the record**, one file set: `crates/nvs-cli/src/check.rs`,
`crates/nvs-cli/tests/check.rs`, `crates/nvs-render/src/json.rs`. Its document today has `labels`,
`notes`, `help` and per-span `utf16` positions that the record does not carry yet.

- [ ] `nvs check --json` renders the record (`rule:ide/check-json-is-the-diagnostic-record-as-a-document`). `crates/nvs-cli/src/check.rs:158`, `crates/nvs-cli/src/check.rs:372`, `crates/nvs-diagnostics/src/record.rs:48`
- [ ] One fixture's whole `--json` document is frozen byte for byte. `crates/nvs-cli/tests/check.rs:1`, `crates/nvs-cli/tests/ast.rs:184`
- [ ] The module doc of `crates/nvs-cli/tests/check.rs` says why both kinds of test exist. `crates/nvs-cli/tests/check.rs:6`

Stage 3 has no gate, shares no file with stage 4, and fits beside it if the session has room.

### Backlog

- A published JSON schema for `check --json` is not planned: `nvs ast --json` publishes none either (`docs/rules/ide/ast-json-schema-is-frozen.md`).
