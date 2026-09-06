`inout` invites the question of a write-only `out` mode: assigned before return, unreadable on entry. There
is none.

It is a separate feature needing its own definite-assignment analysis pointed at parameters rather than at
properties, and nothing asks for it. This is recorded so it is not reopened as a corollary of the `inout`
spelling.
