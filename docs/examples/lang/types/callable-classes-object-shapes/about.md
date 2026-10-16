The types for things that are not plain values: a function you can pass around, a class, any
object, and an object described by the fields it has.

A `callable` is an anonymous function written with `fn`, or a method reference such as
`Core\Str::length(...)`. A class, interface or enum name is a type, and `object` is the type every
instance fits. A shape describes an object by its fields. `as Point` checks that a value has those
fields and throws an error if not. `is Point` gives `true` or `false`.

To pick a class while the program runs, convert its name with `as class<T>`. A name from a setting
is checked when it runs, and `as ?class<T>` gives `null` for a wrong name.

**Good to know:** a call through a `callable` returns a `mixed` value, so convert the result where
you use it: `$f(4) as int`.

**The examples below** pass a pricing rule around, check a value against a shape, and pick a
report class from a name in quotes and from a setting.
