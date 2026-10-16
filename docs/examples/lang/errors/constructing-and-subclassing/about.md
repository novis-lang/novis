Every error is built the same way: `new` with a message, and an optional second argument naming the
error that caused this one.

You can write an error class of your own. It extends `Throwable` or any class under it. A class that
writes no constructor uses the one it inherits, so `new OutOfStock("no mugs left")` works with
nothing else to write. A class that wants more can write its own constructor, add properties, and
pass the message on with `parent::constructor($message)`.

A message is always required. `new RuntimeError()` does not compile.

**Good to know:** both arguments have names, so you can write
`new RuntimeError(message: "the server said no")` and read the call without counting positions.

**The examples below** show an error class of your own, one that adds properties in its own
constructor, and a loader that keeps the error it caught as the cause of the error it throws.
