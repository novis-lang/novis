Creates an object when the name of its class is in a string.

`construct($arguments)` creates a new object of the described class and runs its constructor.
`$arguments` is a list with one value for each constructor parameter, in order. Use `[]` for a
class with no constructor parameters. The result has the type `mixed`, so convert it with `as`.

`construct` follows the same rules as `new` at the same place in your code. A private constructor
cannot be used from outside its class. Too few arguments, or an argument of the wrong type, throws
a `LogicError` and no object is created. It replaces PHP's `ReflectionClass::newInstanceArgs`, and
there is no way to switch the visibility check off.

**The examples below** show an object created from a class name, the errors for a private
constructor and a missing argument, and a program that creates the handler its configuration
names.
