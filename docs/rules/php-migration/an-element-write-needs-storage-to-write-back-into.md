Copy-on-write separates the array before the element is written, and the separated copy has to land
back in whatever held the array. A temporary — a call's result, an array literal, a conditional —
holds it nowhere, so `f()[0] = 2` is refused where it is written (`E0700`). In PHP the write lands
in the temporary and is discarded: no warning, no notice, nothing observable. The only statement this
costs is one that could not have done anything, so no program that ran is lost.

Every root that *is* storage is unaffected: a local, a property, a static property — and the
receiver under a property is evaluated exactly once, PHP's own count, however deep the chain and
whichever spelling writes it. A temporary receiver's property is included, and that is the one
direction this runs the other way: PHP 8.5.9 refuses `(new Box())->rows[0] = 2` at compile time
(*"Cannot use temporary expression in write context"*) while accepting `make()->rows[0] = 2`, and
Novis accepts both, the field being a slot in a heap object either way. Accepting where PHP refuses
loses no program that ran. Parentheses are transparent on both sides — `($a)[0] = 2` writes `$a[0]`
here exactly as it does in PHP.
