`nvs ast --json` has a frozen schema, because the AST panel needs a stable shape and `{stmts:#?}` — Rust's
derived `Debug` — has none: any field reordering in any AST struct changes it. The schema is a node
object of `kind`, `span` as `[start, end]`, the node's own scalar fields, and `children`. Trivia and
recovery nodes are included, because a panel that hides them is least useful on exactly the file the
developer is looking at the panel to understand, and the resilient tree is the default, with
`--strict` the opt-in: the panel's whole value is on a file that does not compile. `crates/nvs-cli/tests/ast.rs` checks the schema over every
file in `examples/` and freezes one example's whole document byte for byte.

No literal's text is a field (`crates/nvs-syntax/src/walk.rs`). The span names it, so a consumer
holding the file already has it, and a consumer that does not hold the file is never handed a `secret`
literal's bytes. The output is a parse alone, and the qualifier is a type, so leaving the text out is
how `--json` keeps `rule:security/redaction-reaches-the-tools-own-renderings` without a type check. A
renderer that carries a literal's text must first have type-checked it and print the placeholder
`rule:security/secret-sinks-refuse` gives a dumped property in place of a `secret` one.
