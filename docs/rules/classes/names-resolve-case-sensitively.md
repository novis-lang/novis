Every name — class, interface, enum, enum case, namespace segment, method, property, parameter, local,
class constant — is compared exactly, everywhere resolution happens. `Foo` and `foo` are two names.
There is no configuration and no compatibility mode.

Folding case for some categories and not others is a split nobody memorizes, and then the one place
that compares a class name as a string — a router table, a cache key, a serialized payload —
disagrees with the resolver. It costs nothing to be strict, because exactly one casing is legal per identifier category anyway: two
names differing only in case cannot both be valid declarations of the same kind, so case-sensitive
resolution can never make a working program ambiguous. It only turns a wrong reference into a
diagnostic.

A reserved-name check that is deliberately case-insensitive is not an exception to this. It only ever
rejects more, and a tightening cannot make a program's meaning depend on case, because no spelling it
touches has a meaning to depend on.
