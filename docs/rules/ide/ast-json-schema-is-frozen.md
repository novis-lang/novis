`nvs ast` gains `--json`, because the AST panel needs a stable shape and `{stmts:#?}` — Rust's derived
`Debug` — has none: any field reordering in any AST struct changes it. The schema is a node object of
`kind`, `span` as `[start, end]`, the node's own scalar fields, and `children`. Trivia and recovery nodes
are included, because a panel that hides them is least useful on exactly the file the developer is looking
at the panel to understand, and `--resilient` is the default: the panel's whole value is on a file that
does not compile. A snapshot test over `examples/` freezes it.

One scalar field is not the source text, and it is the only place this output depends on anything past the
parse: a literal node whose static type carries `secret` emits the fixed placeholder
`rule:security/secret-sinks-refuse` gives a dumped property, rather than its own bytes
(`rule:security/redaction-reaches-the-tools-own-renderings`). Putting that in the JSON rather than in the
panel is what stops `--json` and the webview from disagreeing about it.
