`Revocable` is the interface for an object that can stop being valid, such as a deleted database row.

To revoke an object means to tell the program that the object is no longer valid. A class that
implements `Revocable` gets two methods. `revoke()` revokes the object, and a second call does
nothing. `isRevoked()` returns `true` after `revoke()` was called.

After `revoke()`, every `Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap` removes the object
immediately. Their `count()` is correct, and a running `foreach` skips it.

A plain array, a property or a variable still contains the object. On a revoked object, reading a
property, calling a method, `clone` and `Core\Json::encode` throw `RevokedObjectError`. The message
names the class and the line of the first `revoke()`. `==`, `is` and `::class` do not throw.

**Good to know:** only a class with no parent class can write `implements Revocable`. All its
subclasses are then revocable too.

**The examples below** show an identity map that forgets a deleted row, a set of online users, and a
job heap.
