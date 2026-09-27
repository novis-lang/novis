`Core\Serialize::encode()` turns a value into `bytes`, and `Core\Serialize::decode()` builds the same
value again from them. It replaces PHP's `serialize()`.

The value can be a number, a string, an array or an object. Arrays and objects can contain other
arrays and objects. If two places point to the same object, they still point to one object after
`decode()`. You can save the bytes in a file or a cache, or keep them as a copy of a value at one
moment.

Some values cannot be turned into bytes: a closure, an open file or connection, and an object with a
`secret` property. For these values, `encode()` throws a `LogicError`.

**Good to know:** the bytes use a format that only Novis reads. PHP cannot read them, and Novis cannot
read the output of PHP's `serialize()`.

**The examples below** show a list turned into bytes and back, two orders that share one customer, and
a copy of a shopping cart that undoes a change.
