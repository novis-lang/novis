Returns the parameters of a method, in the order they are written.

`parameters()` returns an array with one `Core\Reflect\ParameterInfo` for each parameter. `$this`
is not included. Each `ParameterInfo` gives the name of the parameter with `name()`, without the
`$`, and its declared type with `type()`.

For a method that the compiler adds, such as the constructor of `LogicError`, the array is empty.
`Core\Reflect\MethodInfo::parameterCount` still returns how many arguments that method takes.

**The examples below** print the parameters of a method, print their types, and check a web form
for the fields that a handler needs.
