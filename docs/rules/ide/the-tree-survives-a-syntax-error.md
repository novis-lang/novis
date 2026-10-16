A live editor spends most of its time on a syntactically invalid document — mid-statement, an unclosed
brace, a half-typed identifier — so completion and hover that go dark on the first error rarely work
when they matter. The parser therefore never fails to produce a tree: every `parse_*` returns a node
rather than a `Result`, a missing token is reported at the empty span where it should have been without
consuming what follows, and `$u->` with nothing after it parses to a property access whose name was
synthesized at the cursor.

Recovery is explicit, never inferred. A node the parser invented says so — `MemberName::Missing`, a span
on `ExprKind::Error` naming what it stood in for — because completion's whole behaviour turns on telling
a name the user wrote from one the parser made up at the cursor, and an empty span cannot carry that.

One walk builds a `SyntaxIndex` answering "the innermost node at this byte offset, and its ancestors",
so the server maps a cursor back to a syntax node even inside a malformed region, with no second
position-mapping mechanism. The test is direct: an unclosed brace or a trailing `->` does not stop
completion on the well-formed code around it. What this gives up is incremental reparse — every analysis
reparses the document — and a latency bound keeps that a measured trade.

**Declined:**
- **Completion stops at a syntax error until it is fixed.** A document is invalid for most of the time
  somebody is typing in it, so completion would rarely work when it is wanted.
- **Recovery layered over an all-or-nothing parser afterwards.** A parser that gives up produces no tree
  for anything downstream of the error to be layered on; the tree has to come from the parser itself.
