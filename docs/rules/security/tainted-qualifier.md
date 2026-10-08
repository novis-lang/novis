`tainted string` and `tainted bytes` join the type grammar as a qualified form of the two scalar
types, and `tainted {…}` writes the same fact over a whole shape of them. It is not a class, not a
wrapper and not a run-time tag: it is checked once, while checking, and carries no representation past
that point — no extra byte in the value's header, no refcount change, nothing on the hot path.

**Over a shape it distributes and then disappears.** `tainted {a: string, b: {c: bytes}}` is rewritten
while parsing to the shape whose text-carrying fields are their tainted forms, transitively — through a
nested shape, a nullable, a union member and an `array<string>` element — so it produces exactly the
field-by-field spelling it saves, and no layer past the parser knows it was written. A shape carrying no
`string` and no `bytes` anywhere is a diagnostic rather than a no-op: a qualifier that promises nothing
still reads as a promise.

**`mixed` is not qualifiable, and that is the same boundary.** The checker cannot distribute a qualifier
through an erased container, so `tainted mixed` would promise what nothing enforces; structured input
stays `array<mixed>`. The qualifier is recovered where the payload is named instead: text out of `mixed`
is `tainted` however it is taken out (`rule:security/taint-propagation`), and a request body converted
into a shape is converted into `tainted {…}` — not by qualifying the container it arrived in.

**It is grammar, not only a type-checker fact.** Every binding carries a written type
(`rule:types/declaration`), so a function that receives a tainted value and passes it on has nowhere
to put that fact unless `tainted` can be spelled in an ordinary declaration. Without it, taint would
disappear silently at the first call boundary — which is the hole the qualifier exists to close — or
every request-handling function would have to launder on its first line.

`tainted` is written after `secret` when both appear, and `tainted secret string` is a diagnostic
naming the required order rather than a second spelling (`rule:security/secret-qualifier`).
