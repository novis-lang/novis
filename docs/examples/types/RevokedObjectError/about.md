The error a use of a revoked object throws. An object is revoked when its class implements
`Revocable` and the program called `revoke()` on it. The error is a `LogicError`, because the program
still uses an object that is no longer valid.

A property read or write, a method call, `clone`, `Core\Json::encode` and `Core\Reflect` throw it.
`isRevoked()`, `==`, `is` and `::class` do not. Its message names the class and the file and line of
the first `revoke()`: "This `Blog\Post` object was revoked in `src/Blog/Post.nvs` on line 42. A
revoked object cannot be used."

You can check first with `isRevoked()`, and then no error is thrown.

**The example below** shows a deleted post that is still in a plain array, and the error that a read
of its title throws.
