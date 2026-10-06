An object reaches a string only through the global interface `Stringable`, whose single member is
`toString(): string`. Every implicitly converting position — interpolation, concatenation, `echo` and
`print`, and an `as string` conversion — accepts an object only when its static type provably
implements it, and calls `toString()`. An object whose class does not is a compile-time diagnostic
naming `Stringable` as the fix.

`Stringable` lives in the global namespace, not under `Core`: it is a contract an ordinary class
implements, not a domain class holding `static` members. The method is `toString`, not `__toString`,
for the same reason `PropertyObserver`'s members are not `__get` — a magic spelling would misdescribe
what the declaration site is doing.

The refusal is made wherever the static type names a class, which is the whole of what a compile-time
rule can promise. Through a `mixed` or a plain `object` the same question is answered from the
instance's runtime class and a class with no `toString` throws there, because there was no site to
refuse at.
