A class's declared properties are its whole property surface. Naming one that is not on the list is a
compile-time diagnostic where the name is a literal identifier, and a checked, catchable throw where
the name arrives at run time — through reflection, through an erased receiver
(`rule:types/erased-member-access`), or through a property key. A write through any of those can never
create a field, and it is checked against the field's real declared type.

`PropertyObserver` is never consulted for a name that does not exist. There is no path in Novis from "the name is wrong" to any user
code running at all, which is what makes a typo a failure rather than a silent second property.

A name that is computed out of nothing is refused rather than deferred: `$obj->$name` and
`$obj->{$expr}` do not resolve, in front of a call's parentheses as much as on a property. The one
operand that carries its own answer is `rule:types/property-key`'s `property<T>`, whose value is by
construction one of `T`'s public declared names — the check moved to the conversion, once, instead of
being repeated at every access.
