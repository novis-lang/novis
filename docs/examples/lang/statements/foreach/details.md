One header can use `var` for one variable and a written type for the other. A variable without
either is the error `E0101`.

`var` on the value keeps every qualifier of the element type. Over an `array<tainted string>`, the
value variable is a `tainted string`.

A written type can be wider than the element type. Write `int|float $v` over an `array<int>` if the
body stores a `float` in `$v`. The compiler checks that each element fits the written type.

A generator, an `Iterator<T>` and an `Iterable<T>` give values of type `T` and have no key. A key
variable over one of them is the error `E0444`.

The word `as` in the header belongs to `foreach`. To convert the subject, put the conversion in
parentheses: `foreach (($m as array<int>) as var $v)`. A `mixed` subject without a conversion is the
error `E0443`.

An array literal can be the subject when the value variable uses `var`. All its elements need the
same type. Elements of different types are the error `E0414`, and so is an empty array `[]`. With a
written type, an array literal does not compile. Put it in a typed variable first.

An `inout` loop walks the array itself. An element that the body adds at the end is visited too.

A later `foreach` in the same function can declare the same variable again with the same type. With
another type, the second declaration is the error `E0406`.
