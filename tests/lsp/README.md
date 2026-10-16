# The `.lspt` tree — one editor answer per file, frozen

A case here is a document, a `<|>` cursor, a request and the answer rendered as text
(`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`). `bun nv peek rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`
is the rule; **the format itself is `crates/nvs-lsp/src/lib.rs`'s module doc**, which is its one home, and
[docs/agent/conventions.md](../../docs/agent/conventions.md) § *An `.lspt` case* is the shape to copy.

`nvs lsp-test tests/lsp/` runs the tree and prints `N passed, M failed`. A new file here is picked up with
no registration, and its coverage is inferred from the node its cursor resolved to — nobody maintains a
list (`rule:ide/lspt-coverage-is-inferred`).

**This is not `nvs test`.** The two suites answer different questions and share no summary: `.nvst` runs a
program and freezes its stdout, `.lspt` asks a question of a document that usually does not even parse,
and that is the point — a case that only ever asks about valid code is not testing what the resilient tree
exists for.

Every request slice ships its own cases, so the tree fills in with the handlers rather than ahead of them.
`--EXPECT--` is frozen: a case's document, cursor or request may be corrected freely, its expectation is
never edited to make it pass.
