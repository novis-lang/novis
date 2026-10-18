A default value is not a write, so it does not call `beforePropertySet`. A write of a property the
class declares calls it, and so does a write of a property a subclass declares. A write through a
`mixed` variable calls it too. A `static` property calls no guard.

`$value` has the type `mixed`. To use it as an `int`, test it first with `$value is int`. A plain
`int $n = $value;` is the error `E0401`.

For a nested element, `$keys` lists every key from the property down to the element. So
`$a->cells["r"][0] = 9` gives `["r", "0"]`. A compound assignment and an increment give the new
value in `$value`. When `$a->x` is `1`, `$a->x += 4` gives `5`, and then `$a->x++` gives `6`.

A guard may write another property of its own class. That write calls the guard again, with the
other property's name. Test `$name` first, or the program calls the guard until the call stack is
too deep.

When `beforePropertyUnset` throws an error, `unset` removes nothing, and the error goes to the
caller.
