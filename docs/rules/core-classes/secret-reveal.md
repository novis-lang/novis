`Core\Secret::reveal(secret string $value, string $reason): string`, with a `bytes` overload, is the
one narrow way out of the `secret` qualifier outside a checked conversion. It is forbidden by
default, rare, greppable, and carries a written reason at the call site, and the reason reaches no
byte of the answer.

There is deliberately no generic `unwrap()` or `expose()`. A catch-all invites false confidence, and
the whole value of a qualifier is that removing it is visible where it happens.

A second, more common removal path is a **purpose-built function that consumes a `secret` and returns
a genuinely non-secret derivative** — password hashing is the canonical case: it takes a
`secret string` and its output is not confidential in the same way, so it may declare a plain
`string` return. That is not a loophole; it is the ordinary shape of "the secret goes in, something
safe to keep comes out", and each such function's author carries responsibility for it being true.

`reveal` removes `secret` and nothing else: a value that was also `tainted` stays `tainted`.
