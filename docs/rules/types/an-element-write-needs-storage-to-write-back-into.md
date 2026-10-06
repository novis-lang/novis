An element write through a temporary is refused, because copy-on-write separates the array before
the element is written and the separated copy has to land back in whatever held the array. A
temporary — a call's result, an array literal, a conditional — holds it nowhere, so `f()[0] = 2` is
refused where it is written (`E0700`). The write could not have been observed by anything, so the
refusal costs no program that did something.

Every root that *is* storage is unaffected: a local, a property, a static property — and the receiver
under a property is evaluated exactly once, however deep the chain and whichever spelling writes it.
A temporary receiver's property is storage too: `(new Box())->rows[0] = 2` and `make()->rows[0] = 2`
are both accepted, the field being a slot in a heap object either way. Parentheses are transparent —
`($a)[0] = 2` writes `$a[0]`.
