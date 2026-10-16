Tells you whether code outside a class may read and write one of its properties.

`isPublic()` returns `true` for a `public` property. It returns `false` for a `protected` or
`private` property. Those properties are still in the list that
`Core\Reflect\ClassInfo::properties` returns, so you can see that they exist.

If `isPublic()` returns `false`, `Core\Reflect\ClassInfo::get` and `Core\Reflect\ClassInfo::set`
throw a `RuntimeError` when you call them from outside the class. Checking `isPublic()` first lets
your program skip those properties.

**The examples below** print which properties of a class are public, read only the public
properties of an object, and list the fields that a web API may send back to a client.
