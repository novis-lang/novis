Narrowing is flow-sensitive and **branch-local**, and there are five spellings of it: `is`,
`instanceof`, a `== null` test, a comparison against a literal-typed value, and `match (true)`. A
`switch (true)` narrows per arm the same way. A write inside a narrowed block widens the binding
again, because the narrowing described the value that was there, not the slot.

`is` is the general one — it tests a value against any type a value can inhabit, where `instanceof`
tests only a class (`rule:types/type-test` owns both the accepted set and why the two coexist). Every
spelling narrows on the **true edge alone**. Subtracting a union member on the failing edge is
deliberately not done by any of the five: it is a separable improvement, and one that has to be taken
for all of them at once or not at all.

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
