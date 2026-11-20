`Revocable` is the interface for an object that can stop being valid, such as a deleted database row.

A class that implements it gets two methods. `revoke()` revokes the object, and a second call does
nothing. `isRevoked()` returns `true` after `revoke()` was called.

After `revoke()`, every `Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap` removes the object at once.
Their `count()` is correct, and a running `foreach` skips it.

A plain array, a property or a variable still contains the object. Reading a property, calling a
method, `clone` and `Core\Json::encode` then throw `RevokedObjectError`. The message names the class
and the line of the first `revoke()`. `==`, `is` and `::class` do not throw.

**Good to know:** only a class with no parent class can write `implements Revocable`. Its subclasses
get it too.

**The examples below** show an identity map that forgets a deleted row, a set of online users, and a
job heap.
