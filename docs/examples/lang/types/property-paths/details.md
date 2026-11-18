Every name in a path but the last must be a property whose type is a class `C`, a `?C`, an
`array<E>` or a `?array<E>`. After a class, the next name is a property of `C`. The last name can be
any public property.

After an array, the next part is `@key`, `@value` or a property of the element class:

- `@key` is the key of an element. Its type is `int|string`, and the path ends there.
- `@value` is the element, of type `E`. The path can end there. If `E` is a class, the path can go on
  to its properties. If `E` is an array, the path can take `@key` or `@value` again, such as
  `grid.@value.@key`.
- A property of the element class directly after the array is a shorter way to write `@value` and
  that property. `authors.name` is the same path as `authors.@value.name`.

A property name never starts with `@`. So `authors.value` is always the property `$value`, and
`authors.@value` is always the element. Any other word after `@`, such as `@index`, is the error
`E0405`. So is `@key` after a property that is not an array, and any part after `@key`.

`Core\Reflect::steps` returns one entry for each part of the path. The entry for `@key` is named
`@key`, with the type `int|string`. The entry for `@value` is named `@value`, with the element's type,
such as `Author`. Both entries are public. A shorter path such as `authors.name` has no `@value` entry.

A path written directly in the code is checked while compiling, one name at a time. This is true
for `"customer.nme" as propertyPath<Order>` and for a string literal at a parameter typed
`propertyPath<Order>`. A wrong name is the error `E0405`, and the error points at that name. A path
that continues after a property that is not an object or an array, such as `total.x`, is also `E0405`.

`as` checks any other string while the program runs. If the path is wrong, it throws a
`RuntimeError`. The message names the class and the part that is wrong. The empty string and a path
that ends with `.` throw too, because they have an empty part.

A path is not used to read a value. `$order->$path` does not compile (`E0235`). Use a
`property<T>` key to read or write one property.

`$path as string` gives the text. Two paths are equal when their text is equal. A path written in
the code and a path converted from a variable are equal when the text is the same.
