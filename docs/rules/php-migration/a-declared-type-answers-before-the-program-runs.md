`$i->name` where `$i` is declared `int` is refused where it is written (`E0495`); PHP warns *"Attempt
to read property"* and yields `null`. A union naming no single class takes the same code, having no
one property set to resolve against. `1 instanceof Box`, or `instanceof` over a declared scalar, an
`array<T>`, an enum or a union naming no class, is refused the same way (`E0497`); PHP answers
`false`, having no declaration to read. The declaration already answered, so the test is dead code
that reads as a live question — the same call `rule:expressions/disjoint-comparison-refused` makes
for `==` over two statically disjoint types.

`mixed` is the exception and keeps PHP's timing: it is the one unchecked position, so `$m->name`
defers to `rule:types/erased-member-access`'s name-keyed fetch, which throws — in PHP's own wording —
for a receiver that turns out not to be an object and for a name its class does not carry. `mixed`,
`object`, a shape and any union holding a class keep the run-time `instanceof` test, and every
non-object tag answers `false` there exactly as PHP does.

The class side is not a divergence. PHP's dynamic `$x instanceof $name` is spelled over a class
reference — `$x instanceof $cls`, where `$cls` is a `class<T>` (`rule:types/class-reference`) — and
tests the class that value holds; a bare `string` on the right is `E0496`, the name having been
checked at the `as` that produced the reference, not at the test.
