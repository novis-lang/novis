A written anonymous object is checked by asking which arm accepts it, where an arm accepts when the object's
keys are exactly that arm's required keys plus any subset of its optional ones, each value assignable to
its field's declared type. **Exactly one arm accepts.** Zero accepting is the call site's error; two
accepting is a registry bug rather than a program's, so it is refused statically — a build-time check
proves for every pair of arms that some field they both declare has non-overlapping types, or that one arm
requires a key the other does not declare at all.

Nothing declares a discriminant field. A settings shape separates its arms because the enum-case types
(`rule:types/literal-types`) on its driver field make one driver and the rest disjoint sets, and
disjointness is all arm selection ever needed. A rule naming a discriminant would be a second, weaker way
of saying the same thing, and it would have nothing to say about a future union separated by a required key
instead of by a value.

What a caller sees is that a key belonging to the arm the other values did not select is refused where it
is written, naming the arm's own key set rather than the merged one.
