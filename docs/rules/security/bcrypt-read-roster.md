Verification accepts exactly two stored shapes: the modern hash this library writes, and a bcrypt hash
under three prefixes — one algorithm under three tags, verified identically. **The roster grows only
by amending this rule, never by accepting what a parser happens to read.**

One further prefix is refused: it exists to be bug-compatible with an implementation's
sign-extension overflow, and verifying it means reimplementing the bug.

This is what lets an existing user table, hashed by another runtime, verify on day one and rewrite itself one
successful login at a time (`rule:security/needs-rehash-answers-weaker`). What it spends: a few
kilobytes transiently per verification of a legacy row, on the calling task — bounded by in-flight
logins, and shrinking as the table converges.
