Tells you what kind of value a variable contains while the program runs.

A variable of type `mixed` can contain any value: a number, a string, an array, an object or `null`.
`Core\Reflect::typeOf` returns one case of the enum `Core\Reflect\TypeKind`, such as `Int`, `Text` or
`Array`. Every value matches exactly one case. The answer is about the value in the variable now, so
the same variable can give a different case after you assign something new to it.

On a variable with a narrower type, such as `int`, the compiler already knows the answer. `typeOf` is
useful where the type is `mixed`.

**The examples below** show a label for each kind of value, one variable that changes kind, and a
check of configuration values before a program uses them.
