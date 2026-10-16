All five relational spellings lower to one call: `$a < $b` is `$a->compareTo($b) < 0`, `$a >= $b` is
`$a->compareTo($b) >= 0`, and `$a <=> $b` is the call itself. The lowering happens at compile time
and produces an ordinary method call — devirtualized when the static type is known exactly, like any
other call — so no second dispatch path exists beside the one every method already uses.

One method therefore fixes all five operators at once, and they cannot disagree with each other the
way five separate hooks could. A subclass that overrides `compareTo` changes how a base-typed pair
answers, because the call reaches the receiver's own implementation like any other virtual call.

A `compareTo` that throws propagates as a checked status (`rule:errors/propagation`), so an ordering
is a call site with a failure edge rather than an operator that cannot fail.
