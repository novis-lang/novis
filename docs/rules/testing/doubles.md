A double is a shape of closures, structurally checked against an interface:
`Core\Test::double<Clock>({now: fn(): Instant => ...})` **is** a `Clock` and may be passed wherever
one is taken. A method the interface does not declare and a method of it the double leaves
unimplemented are each a compile error. `Core\Test::partial<T>($real, {...})` overrides named
methods and delegates the rest to a real implementation.

No class is generated, no source is evaluated, and nothing the author never wrote appears in a
backtrace. There is no builder, no matcher mini-language and no notion of a "nice" or "loose"
double — that last one is not a choice, since a double of `now(): Instant` has nothing legal to
return by default. Every double is strict because nothing else is expressible.
