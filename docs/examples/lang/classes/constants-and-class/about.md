A class can hold named values that never change, and every class knows its own name.

A constant is written `public const int MAX = 3;` inside the class, with its type. Code inside the
class reads it as `self::MAX`, and code anywhere else as `Limits::MAX`. Both put the value in place
where you read it, so they cost nothing at run time. That is also why the value has to be written
directly in the code, as one value or a list of values.

A subclass can give a constant a new value. In a method, `static::MAX` reads the value of the class
the call was made on, and `self::MAX` reads the value of the class the code is written in.
`static::` works only for a single value, such as a number or a string, and not inside an anonymous
function.

`Limits::class` gives the name of a class as a string. `$order::class` gives the class a value
really is, and `static::class` gives the class the call was made on.

**Good to know:** a constant that holds a list gives you a new list on every read. If you change
the list you got, the next read is unchanged.
