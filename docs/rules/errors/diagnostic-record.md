Every developer-facing output in Novis is a **record**: an envelope plus a tree of **nodes**. The
model is closed — a node is one of a fixed set of kinds, and there is no extension point.

The envelope carries `ts`, `level`, `message`, `request_id`, `trace_id`/`span_id` when a trace is
active, `source`, `count`, and `fields` (`rule:errors/log-fields`). A node is a Scalar tagged with
its Novis type — so `"1"` and `1` are never confusable, which is the one thing `print_r` cannot do —
or a Sequence, a Map, an Object with its *declared* properties, an Enum case named rather than
numbered, a Callable's signature without its body or captures, a Redacted, an Elided, a Cycle
carrying the identity of the node it repeats, or a Span over a source range.

**The model is content, not presentation.** It carries no colour, no indentation, no width and no
ordering-for-display; a rendering supplies all four. That separation is what makes
`rule:errors/record-transformations` decidable once, and it is why a fourth rendering would cost one
implementation rather than five.

One crate — `nvs-render` — owns the model and every rendering of it, and both the runtime and the
compiler front end depend on it.

Closing the model is what keeps the renderings from drifting apart. The obligation on them is a table
of every node kind against every rendering, and that table is finite only because no producer can add
a kind.

- **Memory:** a record is built before it is rendered, so each log write or dump holds one tree of
  nodes rather than formatting straight to bytes: one extra allocation and one extra walk per record,
  bounded by the elision caps (`rule:errors/record-transformations`) and charged to the request. It
  buys a dump that cannot reach a production response and a plaintext log target that cannot be
  forged.
