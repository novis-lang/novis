If a class has a parent class and writes `implements Revocable`, the compiler reports the error
`E1001`. The help of the error names the top class of the hierarchy, the one with no parent. Write
`implements Revocable` on that class instead.
An interface that extends `Revocable` is the error `E1002`. A class that declares its own `revoke()`
or `isRevoked()` is the error `E1003`. An abstract class with no parent may implement `Revocable`.

Every use of a revoked object throws `RevokedObjectError`. `RevokedObjectError` is a `LogicError`.
These uses throw:

- a property read or write, `.=`, `+=`, `++` and `--` on a property, and `isset`, `empty` and
  `isinit` of a property;
- an `inout` property argument, before the call and again when the value is written back;
- a method call, `?->` included, and a `static` method that you call through the object;
- a method that the class delegates to a property, and a call of a method reference;
- a conversion to a string, such as a variable inside a double-quoted string, a concatenation or
  `echo`;
- `<`, `<=`, `>`, `>=` and `<=>`, which call `compareTo`;
- a `foreach` over the object;
- `clone`, `Core\Json::encode`, and a `Core\Reflect` read, call or write.

These do not throw: `isRevoked()`, a second `revoke()`, and passing, storing or returning the
object. `yield`, creating a method reference, `==`, `match` and `switch` do not throw either. The
type test `is`, `as` to a type the object has, and `::class` do not throw.

The error message has this form: "This `Blog\Post` object was revoked in `src/Blog/Post.nvs` on line
42. A revoked object cannot be used." A second `revoke()` does not change the line. A call of
`revoke()` through `mixed` or `Core\Reflect` has no line in the source code. Then the message names
only the class.

`Core\Serialize::encode` of a revoked object throws too. So does sending it to a task or storing it
in a cache. Each of these throws the same error class as for any other value that it cannot copy.
The message is the message of `RevokedObjectError`.

`Core\Debug::dump` does not throw. It prints the class name and `(revoked)`, and no properties.
For an object that is not revoked, it prints the properties of the class and nothing else.

In this section, a collection is a `Core\ObjectMap`, a `Core\ObjectSet` or a `Core\Heap`.

`clone` of an object that is not revoked gives a new object. The new object is not revoked. It is
in no collection, even when the original object is. `Core\Serialize::decode` also gives a new object
that is in no collection.

`set`, `add` and `push` of a revoked object do nothing. So a collection never contains a revoked
object. When the memory limit stops a `set`, `add` or `push`, the object is not added to that
collection. If the object is revoked later, `revoke()` works as usual.

`Core\ObjectMap<string, Post>` is not a `Core\ObjectMap<string, Storable>`, even when `Post` extends
`Storable`. If it were, a program could add a `Storable` that is not a `Post` to a map of posts.

A static property cannot have a `new` object as its default value (`E0472`). Give the property a
nullable type, and create the map the first time you need it:
`self::$objectsByKey ??= new Core\ObjectMap<string, Storable>();`

**Speed and memory.** A property read or a method call through a revocable class, an interface or
`mixed` runs one extra check. Through any other class type there is no check. Each revocable object
uses 16 more bytes. The first time it is added to a collection, it uses 24 more bytes. Each
collection that contains it adds another 32 bytes. `revoke()` takes longer when more collections
contain the object. The size of each collection does not change its speed. The memory belongs to the request and is freed when the request ends.
