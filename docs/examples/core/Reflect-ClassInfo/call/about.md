Calls a method of an object when the method's name is in a string.

`call($object, $name, $arguments)` runs the method `$name` on `$object` and returns its result.
`$arguments` is a list with one value for each parameter, in order. Use `[]` for a method with
no parameters. The result has the type `mixed`, so convert it with `as`.

`call` follows the same rules as a normal method call at the same place in your code. A private
method cannot be called from outside its class, and a wrong name or the wrong number of arguments
throws an error. It replaces PHP's `ReflectionMethod::invoke`, and there is no way to switch the
visibility check off.

**The examples below** show a call with two arguments, the errors for a private method and a
wrong name, and a command line tool that runs the method named by a command.
