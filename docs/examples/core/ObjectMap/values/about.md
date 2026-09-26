Returns every value of a `Core\ObjectMap` as a list. The values are in the order their keys were
added. The value at each position belongs to the key at the same position in the list that `keys`
returns.

The list is a new array. If you change the map after the call, the list does not change. If the
map is empty, the result is an empty array. A `null` value is in the list like any other value.

Use `values` when you need the values and not the objects they belong to, for example to add them
up or to pass them to a function that takes an array.

The examples show listing the values, matching each value to its key, and a common use: adding up
the total of all open shopping carts.
