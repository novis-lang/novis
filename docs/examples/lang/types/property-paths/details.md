Every name in a path but the last must be a property whose type is a class `C`, a `?C` or an
`array<C>`. The next name is a property of `C`. The last name can be any public property.

A path written directly in the code is checked while compiling, one name at a time. This is true
for `"customer.nme" as propertyPath<Order>` and for a string literal at a parameter typed
`propertyPath<Order>`. A wrong name is the error `E0405`, and the error points at that name. A path
that continues after a property that is not an object, such as `total.x`, is also `E0405`.

`as` checks any other string while the program runs. If the path is wrong, it throws a
`RuntimeError`. The message names the class and the part that is wrong. The empty string and a path
that ends with `.` throw too, because they have an empty part.

A path is not used to read a value. `$order->$path` does not compile (`E0235`). Use a
`property<T>` key to read or write one property.

`$path as string` gives the text. Two paths are equal when their text is equal. A path written in
the code and a path converted from a variable are equal when the text is the same.
