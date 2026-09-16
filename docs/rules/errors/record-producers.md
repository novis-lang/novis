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
