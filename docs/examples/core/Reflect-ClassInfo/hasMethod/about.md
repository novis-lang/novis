Checks whether a class has a method with a given name.

`hasMethod($name)` returns `true` when the described class has a method called `$name`, and
`false` when it has none. Methods the class inherits from a parent class count too. The name must
match exactly, so `Title` does not find a method called `title`. It replaces PHP's
`method_exists`.

`hasMethod` also returns `true` for a private method. That method still cannot be called from
outside its class. `call` throws a `LogicError` both for a private method and for a name that does
not exist, so `hasMethod` is how a program tells the two cases apart.

**The examples below** check a few names, tell a private method from a missing one, and call an
event handler only on the objects that have it.
