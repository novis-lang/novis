Where a production synthesises a node at the point of a failure, the node says so. A member name the
parser invented at the cursor is `MemberName::Missing(Span)` beside `MemberName::Ident`, and
`ExprKind::Error` carries the span of what it stood in for.

A consumer must never have to guess whether an identifier is one the user wrote or one the parser
invented. Completion's whole behaviour hangs on that distinction — `$u->` with nothing after it parses to
a property access whose name is missing, and that missing name is precisely the node member completion
needs — and an empty span is a coincidence, not a contract.
