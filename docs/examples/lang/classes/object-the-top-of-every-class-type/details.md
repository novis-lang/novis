A method call through `object` is the error `E0477`. This is true for `?->` on a `?object` too.

An `object` variable contains only an object of a class. An `int`, `null` or an anonymous function
at an `object` is the error `E0401`. Write `?object` for a value that can be `null`.

An `object` value at a parameter or a variable of a class type is the error `E0401`. Convert it with
`as` first.

A property read through `object` gives `mixed`. So `int $n = $o->count;` is the error `E0401`. Write
`$o->count as int`.

A write through `object` compiles, and the value is checked while the program runs. A property the
object does not have throws a `RuntimeError`. A value of another type than the property's throws a
`RuntimeError` too, and so does a write to a `readonly` property.
