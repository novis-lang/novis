Reads a property of an object when the property's name is in a string.

`get($object, $name)` returns the value of the property `$name` on `$object`. The result has the
type `mixed`, so convert it with `as`. If the property has a `get` hook, the hook runs, the same
as for a normal read.

`get` follows the same rules as a normal property read at the same place in your code. A private
or protected property cannot be read from outside its class, and `get` throws a `RuntimeError`.
A name that is not a property of the class throws a `LogicError`. It replaces PHP's
`ReflectionProperty::getValue`, and there is no way to switch the visibility check off.

**The examples below** show a read by name, the two errors, and a report that writes only the
columns a user chose.
