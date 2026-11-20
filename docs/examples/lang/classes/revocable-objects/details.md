A class with a parent class that writes `implements Revocable` is the error `E1001`. The help names
the class with no parent, where you write it instead. An interface that extends `Revocable` is the
error `E1002`. A class that declares its own `revoke()` or `isRevoked()` is the error `E1003`. An
abstract class with no parent may implement `Revocable`.

These uses of a revoked object throw `RevokedObjectError`: a property read or write, a method call,
`clone`, `Core\Json::encode`, and a `Core\Reflect` read, call or write. These do not throw:
`isRevoked()`, a second `revoke()`, passing or storing the object, `==`, the type test `is` and
`::class`. `RevokedObjectError` is a `LogicError`.

The error message has this form: "This `Blog\Post` object was revoked in `src/Blog/Post.nvs` on line
42. A revoked object cannot be used." A second `revoke()` does not change the line. When `revoke()`
is called through `mixed` or `Core\Reflect`, the message names only the class.

`serialize()` of a revoked object throws too. So does sending it to a task or storing it in a cache.
Each of them throws its usual error, with the same message.

`Core\Debug::dump` does not throw. It prints the class name and `(revoked)`, and no properties.

`clone` of an object that is not revoked gives a new object. The new object is not revoked, and no
collection contains it.

A revoked object is never added to a collection. `set`, `add` and `push` of one do nothing.

`Core\ObjectMap<string, Post>` is not a `Core\ObjectMap<string, Storable>`, even when `Post` extends
`Storable`. Both variables would contain the same map, and the first one may contain only posts.

A static property cannot have a `new` object as its default value (`E0472`). Give the property a
nullable type, and create the map the first time you need it:
`self::$objectsByKey ??= new Core\ObjectMap<string, Storable>();`

**Speed and memory.** A property read or a method call through a revocable class, an interface or
`mixed` runs one extra check. Through any other class type there is no check. Each revocable object
uses 16 more bytes. Its first collection adds 24 bytes, and each collection that contains it adds 32
bytes. `revoke()` takes time
in proportion to the number of collections that contain the object. The memory belongs to the request
and is freed when the request ends.
