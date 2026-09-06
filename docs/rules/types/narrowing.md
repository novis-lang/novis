Narrowing is flow-sensitive and **branch-local**, and there are four spellings of it: `instanceof`, a
`== null` test, a comparison against a literal-typed value, and `match (true)`. A `switch (true)`
narrows per arm the same way. A write inside a narrowed block widens the binding again, because the
narrowing described the value that was there, not the slot.

Nothing else narrows. In particular an equality against an enum case does not — `$m == Mode::Read`
leaves `$m` at its declared type in the branch it guards, and `$m as Mode::Read|Mode::Write` is how a
case-subset type is reached (`rule:types/enum-case-type`). Adding equality-driven narrowing would
have to be stated again for `!=`, `&&`, `||` and negation, where `as` says the same thing in one
place.

Narrowing never changes a binding's declared type (`rule:types/declaration`); it changes what the
checker knows about it on one path. A value that has to *stay* narrowed is a second binding at the
type you want, or a checked `as` (`rule:types/conversion`).
