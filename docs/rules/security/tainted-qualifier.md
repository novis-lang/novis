`tainted string` and `tainted bytes` join the type grammar as a qualified form of the two scalar
types. It is not a class, not a wrapper and not a run-time tag: it is checked once, while checking,
and carries no representation past that point — no extra byte in the value's header, no refcount
change, nothing on the hot path.

**It is grammar, not only a type-checker fact.** Every binding carries a written type
(`rule:types/declaration`), so a function that receives a tainted value and passes it on has nowhere
to put that fact unless `tainted` can be spelled in an ordinary declaration. Without it, taint would
disappear silently at the first call boundary — which is the hole the qualifier exists to close — or
every request-handling function would have to launder on its first line.

`tainted` is written after `secret` when both appear, and `tainted secret string` is a diagnostic
naming the required order rather than a second spelling (`rule:security/secret-qualifier`).
