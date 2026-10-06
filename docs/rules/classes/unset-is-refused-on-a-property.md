`unset($obj->prop)`, and `unset(Class::$prop)` with it, is a compile-time diagnostic for every declared
property, static or instance, whatever its nullability. Removing the property outright would
leave later access to fall through — an uninitialized-again state that
`rule:classes/definite-property-initialization` exists to make impossible. There is no way to honour
both, and the initialization guarantee is the one that stands. A script wanting a nullable property
back to empty writes `$obj->prop = null;`, an ordinary assignment.

`isset($obj->prop)` is unchanged from what `isset` means for every other binding: a `!= null` test. For
a non-nullable property it is always `true`, and no magic method is consulted — an undeclared name is
already a hard error before `isset` is reached.

Exactly one operand survives: an element of an array held by a local, a property or a static property,
including at depth. Everything else is refused, and the two shapes programs most often write are
named — a local, which is declared once and definitely assigned so there is no undefined state to
return it to, and a temporary, which copy-on-write would separate into a slot nothing can write back.
