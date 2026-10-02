The types for things that are not plain values: a function you can pass around, a class, any
object, and an object described by the fields it has.

`callable` is a function written with `fn`. A class, interface or enum name is a type wherever you
write a type, and `object` is the type every instance fits. A shape describes an object by its
fields, and a value with more fields still fits it. `as Point` checks that a value has the fields
and throws an error if not. `is Point` does the same check and gives `true` or `false`.

To pick a class while the program runs, convert its name with `as class<T>`. A name in a string
literal is checked when the program compiles. A name from a setting is checked when it runs, and
`as ?class<T>` gives `null` for a wrong name.

**Good to know:** a call through a `callable` returns a `mixed` value, so convert the result where
you use it: `$f(4) as int`.

**The examples below** pass a pricing rule around, check a value against a shape, and pick a
report class from a literal and from a setting.
