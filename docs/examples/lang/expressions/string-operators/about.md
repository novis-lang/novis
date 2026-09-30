`.` joins two strings into one, and `.=` adds a string to the end of a variable. Both convert each
side to a string first, so `"Order " . 1042` gives `Order 1042` without a cast.

Every number can be converted this way. `true` gives `1`, and `false` and `null` give an empty
string. An object can be joined when its class implements `Stringable`, and the result of that
method is used.

Four things cannot be converted to a string, and joining one does not compile: a `bytes` value, an
array, an enum case, and a call that returns nothing. The error message says what to write in its
place. For example, you choose the encoding of a `bytes` value or the fields of an array yourself.

A string in double quotes inserts values in the same way: `"Hello $name"` and `"{$order->total}"`.

**The examples below** show strings and numbers joined into one message, a line built piece by
piece with `.=`, and the confirmation text for an order.
