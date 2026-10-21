Five producers build `rule:errors/diagnostic-record`, and none of them implements a format:

| Producer | What it builds |
|---|---|
| `Core\Log::write` | a record with the caller's fields |
| `Core\Debug::dump` | a record at `Debug`, one node per argument |
| a `Throwable` and its trace | a record at `Error`, its nodes one Frame per frame |
| a test result | a record per assertion, expected and actual as sibling nodes |
| a compiler diagnostic | Span nodes over a source map |

Two of them buy something concrete beyond consistency, and they are why the scope is five rather
than two. A test failure renders as a coloured diff locally, as JSON in CI and as HTML in a web
runner with no reporter written for any of them; and `nvs check` gains a JSON rendering the language
server consumes.

The `Throwable` case is the one that would have leaked had the scope been narrower. It is the
most-read diagnostic output in any language, and leaving it outside would have meant a second
implementation of `rule:errors/record-transformations`.

A frame is a **node kind of its own** rather than an Object node, because it is not a class
instance and its two readers want two spellings the shape already has: the JSON rendering writes
`{function, file, line}`, which a pipeline indexes, and the plaintext one writes the `#0`-first
line a person greps. Novis's frames carry a label and nothing else — `rule:errors/propagation`
builds the trace as the throw unwinds — so those three parts are the whole of one, and the only
object a record of a throw carries is the throw itself, whose declared properties are walked like
any other value and redact like any other value.

**Partly on disk.** `Core\Log::write`, `Core\Debug::dump`, an uncaught `Throwable` and a compiler
diagnostic build the record (`crates/nvs-stdlib/src/log.rs`, `crates/nvs-stdlib/src/debug.rs`,
`crates/nvs-runtime/src/floor.rs`, `crates/nvs-diagnostics/src/record.rs`), and a frame is
`nvs_render::Node::Frame`. A compiler diagnostic's terminal text and its `nvs check --json`
document both render from its record (`crates/nvs-cli/src/check.rs`), and its snippet masks a
`secret` literal (`rule:security/redaction-reaches-the-tools-own-renderings`). A test result does
not build the record, and is goal `diagnostic-records`'s gap
`data/gaps/nvs-render/test-result-is-not-a-producer.json`.

**Declined:**
- **Logs and dumps only, with throws, test results and diagnostics outside.** It is the smallest
  scope, and it would duplicate redaction and substitution in each producer left out rather than
  remove the duplication.
