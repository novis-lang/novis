A class can hold named values that never change, and every class knows its own name.

A constant is written `public const int MAX = 3;` inside the class, with its type. Code inside the
class reads it as `self::MAX`, and code anywhere else as `Limits::MAX`. The value is put in place
wherever you read it, so a constant costs nothing at run time. That is also why the value has to be
a literal, or a list of literals.

`Limits::class` gives the name of the class as a string. On a value, `$order::class` gives the name
of the class that value really is, which may be a subclass of the type the variable was declared
with. Inside a method, `static::class` gives the class the call was made on.

**Good to know:** a constant that holds a list gives you a new list on every read. If you change
the list you got, the next read is unchanged.
