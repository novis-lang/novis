The interface for an object that can stop being valid, such as a deleted database row.

To revoke an object means to tell the program that the object is no longer valid. A class that
implements `Revocable` gets two methods. `revoke()` revokes the object, and a second call does nothing.
`isRevoked()` returns `true` after `revoke()` was called.

After `revoke()`, every `Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap` removes the object. A
property read or a method call on the object throws `RevokedObjectError`. `isRevoked()` does not throw.

**Good to know:** only a class with no parent class can write `implements Revocable`. Its subclasses
are revocable too. A class that declares its own `revoke()` or `isRevoked()` does not compile.

**The example below** shows both methods, and the error a read of a revoked object throws.
