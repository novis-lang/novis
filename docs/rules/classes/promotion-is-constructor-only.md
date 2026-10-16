A `public`, `protected` or `private` keyword on a parameter promotes it to a property, and it does
that only in the `constructor`. On any other method's parameter it is refused.

Promotion means exactly this: the keyword says where a *property* may be read from,
and an ordinary method has no allocation to promote into. A property is a slot on an instance, armed
once where the instance is made; a method may be called any number of times, or none, so there is no
moment for a promoted parameter of one to exist.

A promoted parameter is otherwise an ordinary property in every respect — declared, visible,
inherited, initialized by the binding itself
(`rule:classes/definite-property-initialization`), namable by an `implements ... by $field` clause, and
carrying whatever qualifiers its type does.
