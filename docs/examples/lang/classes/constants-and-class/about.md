A class can hold named values that never change, and every class knows its own name.

A constant is written `public const int MAX = 3;`, with its type. Code inside the
class reads it as `self::MAX`, and code anywhere else as `Limits::MAX`. The value must be written
directly in the code, as one value or a list.

A subclass can give a constant a new value. In a method, `static::MAX` reads the value of the class
the call was made on. `self::MAX` reads the value of the class the code is written in. `static::`
works only for a single value, not inside an anonymous function.

`Limits::class` gives the name of a class as a string. `$order::class` gives the class a value
really is.

A constant list gives you a new list on every read.

**The examples below** show `self::` and `static::`, class names as text, a constant list, and
`static::` with a list, which does not compile.
