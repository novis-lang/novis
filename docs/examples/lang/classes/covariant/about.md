`covariant` lets a subclass give a method parameter or a property a narrower type than its parent
gives it. The parent takes any `Animal`, and the subclass takes only a `Dog`.

You write `covariant` before the type: `public function feed(covariant Dog $a)` for a parameter, and
`public covariant Dog $pet;` for a property. Code that uses the parent's type can still pass any
`Animal`. So Novis checks each value when it enters. A parameter is checked before the method body
runs. A property is checked each time it is written. A value of the wrong type throws a
`LogicError` that names both types.

**Good to know:** a narrower type without `covariant` does not compile. The error message tells you
to add it.

**The examples below** show a narrower parameter, the error a wrong value throws, a narrower
property and `compareTo` in a subclass. The last one does not compile.
