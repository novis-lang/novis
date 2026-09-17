Narrowing is flow-sensitive and **branch-local**, and there are four spellings of it: `is`, a
`== null` test, a comparison against a literal-typed value, and `match (true)`. A `switch (true)`
narrows per arm the same way. A write inside a narrowed block widens the binding again, because the
narrowing described the value that was there, not the slot.

`is` is the general one — it tests a value against any type a value can inhabit, and it is the only
type test there is (`rule:types/type-test`). Its value arm narrows too: `$x is $cls`, where `$cls` is
a `class<T>`, narrows the subject to **`T`** on the true edge, which is sound because a `class<T>`
holds `T` or an implementor of it (`rule:types/class-reference-sites`). Every spelling narrows on the
**true edge alone**. Subtracting a union member on the failing edge is deliberately not done by any of
the four: it is a separable improvement, and one that has to be taken for all of them at once or not
at all.

Nothing else narrows. In particular an equality against an enum case does not — `$m == Mode::Read`
leaves `$m` at its declared type in the branch it guards, and `$m as Mode::Read|Mode::Write` is how a
case-subset type is reached (`rule:types/enum-case-type`). Adding equality-driven narrowing would
have to be stated again for `!=`, `&&`, `||` and negation, where `as` says the same thing in one
place.

The subject of every spelling is a **binding**, named. A property, an element or any other place is
never the thing narrowed: `$e->previous != null` proves nothing about the next read of
`$e->previous`, so a nullable one is reached through `?->` or bound to a local and tested there.

Narrowing never changes a binding's declared type (`rule:types/declaration`); it changes what the
checker knows about it on one path. A value that has to *stay* narrowed is a second binding at the
type you want, or a checked `as` (`rule:types/conversion`).
