Every variable in a program has a type, and the type never changes.

The short way to declare a local variable is `var`. `var $count = 3;` gives `$count` the type of its
first value, `int`. In a `foreach` loop, `var` gives the value the type of the array's values, and
the key is a `string`.

You can also write the type yourself, as in `float $weight = 1;`. Write it when the value comes
later, when the variable needs a wider type, or when the type helps the reader. Parameters, return
values, properties and the variable in `catch` always have a written type.

A value of another type does not compile, and neither does reading a variable before it has a value.

**Good to know:** a variable belongs to the whole function, not only to its block. So you can read a
loop counter after the loop.

**The examples below** show `var` and then a written type, a loop counter read after its loop, and an
order where every value has a type.
