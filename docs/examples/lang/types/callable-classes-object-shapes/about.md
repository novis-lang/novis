Types for a function value, a class, any object, and an object described by its fields.

A `callable` is an anonymous function written with `fn`, or a method reference such as
`Core\Str::length`. A class, interface or enum name is a type, and `object` is the type every
instance fits. A shape describes an object by its fields. `as Point` checks that a value has those
fields and throws an error if not. `is Point` gives `true` or `false`.

Where a `class<T>` is expected, write the class as `Invoice::class`. A class name in a string needs
`as class<T>`, and is checked when the program runs. `as ?class<T>` gives `null` for a wrong name.
`new $c()` throws a `LogicError` when the class is `abstract` or an interface.

**Good to know:** a call through a `callable` returns a `mixed` value, so convert the result where
you use it: `$f(4) as int`.

**The examples below** pass a pricing rule around, check a value against a shape, and pick a
report class from a name. Then they get the class of an `object` back, show a shape without a name
as a compile error, and pass a class to a function.
