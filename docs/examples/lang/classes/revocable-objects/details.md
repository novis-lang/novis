If a class has a parent class and writes `implements Revocable`, the compiler reports the error
`E1001`. Its help names the class with no parent. Write `implements Revocable` on that class instead.
An interface that extends `Revocable` is the error `E1002`. A class that declares its own `revoke()`
or `isRevoked()` is the error `E1003`. An abstract class with no parent may implement `Revocable`.

These uses of a revoked object throw `RevokedObjectError`: a property read or write, a method call,
`clone`, `Core\Json::encode`, and a `Core\Reflect` read, call or write. These do not throw:
`isRevoked()`, a second `revoke()`, passing or storing the object, `==`, the type test `is` and
`::class`. `RevokedObjectError` is a `LogicError`.

The error message has this form: "This `Blog\Post` object was revoked in `src/Blog/Post.nvs` on line
42. A revoked object cannot be used." A second `revoke()` does not change the line. A call of
`revoke()` through `mixed` or `Core\Reflect` has no line in the source code. Then the message names
only the class.

`serialize()` of a revoked object throws too. So does sending it to a task or storing it in a cache.
Each of them throws the error class it throws for any value it cannot copy. The message is the
message of `RevokedObjectError`.

`Core\Debug::dump` does not throw. It prints the class name and `(revoked)`, and no properties.

`clone` of an object that is not revoked gives a new object. The new object is not revoked, and no
collection contains it.

A revoked object is never added to a collection. `set`, `add` and `push` of one do nothing.

`Core\ObjectMap<string, Post>` is not a `Core\ObjectMap<string, Storable>`, even when `Post` extends
`Storable`. If it were, a program could add a `Storable` that is not a `Post` to a map of posts.

A static property cannot have a `new` object as its default value (`E0472`). Give the property a
nullable type, and create the map the first time you need it:
`self::$objectsByKey ??= new Core\ObjectMap<string, Storable>();`

**Speed and memory.** A property read or a method call through a revocable class, an interface or
`mixed` runs one extra check. Through any other class type there is no check. Each revocable object
uses 16 more bytes. Its first collection adds 24 bytes, and each collection that contains it adds 32
bytes. `revoke()` takes longer when more collections contain the object. The size of each collection
does not change its speed. The memory belongs to the request and is freed when the request ends.
