Two objects may be compared with `<`, `>`, `<=`, `>=` or `<=>` only when their class implements the
global interface `Comparable`, whose single member is `compareTo(self $other): int` returning
negative, zero or positive. A class that does not implement it cannot be ordered, and the attempt is
a compile-time diagnostic naming `Comparable` as the fix. There is no fallback path anywhere in the
implementation.

`compareTo` is also what every member with a natural ordering reads: `Core\Arr::sort`, `Core\Arr::min`
and `max`, `Core\Math::min`, `max` and `clamp`, and a `Core\Heap` built without a comparator all order
two objects by it and throw for a pair whose class does not implement it. A `Core`-owned instance — a
`Core\BigInt`, a `Core\Time\Instant` — is ordered the same way and through the same one entry point.
An ordering a class states once is the only one anything in the language reads.

PHP walks two same-class objects' declared properties in order and takes the first difference —
behaviour that exists ambiently, that no class opts into or out of, and whose cost is unbounded in
the size of the graph it recurses into. An ordering a class produces should be the ordering its own
code states, once, reviewably.

`Comparable` lives in the global namespace beside `Stringable` and `Parses`, not under `Core`, because
it is a contract an ordinary class implements rather than a domain class holding `static` members. Equality
is a separate question and is unaffected: a `Comparable` class still compares by identity under `==`
(`rule:expressions/object-identity-equality`), and asking the content question explicitly is
`$a->compareTo($b) == 0`.
