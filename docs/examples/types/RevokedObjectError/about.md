`RevokedObjectError` is the error a program gets when it uses a revoked object. An object is revoked
when its class implements `Revocable` and the program called `revoke()` on it. The error is a
`LogicError`, which is the error class for a mistake in the program itself.

Every use of a revoked object throws it. This includes a property read or write, a method call, a
conversion to a string, a comparison, a `foreach`, `clone`, `Core\Json::encode` and `Core\Reflect`.
`isRevoked()`, `==`, `is`, `::class` and passing or storing the object do not throw it. Its message names the class
and the file and line of the first `revoke()`: "This `Blog\Post` object was revoked in
`src/Blog/Post.nvs` on line 42. A revoked object cannot be used."

Call `isRevoked()` before you use an object that may be revoked. `isRevoked()` never throws.

**The example below** shows a deleted post that is still in a plain array, and the error that a read
of its title throws.
