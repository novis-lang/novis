A constant name is SCREAMING_SNAKE_CASE, so `const int max` is the error `E0113`. `static` on a
constant is the error `E0106`. An unknown constant is `E0309`. A constant cannot be assigned:
`A::N = 2;` is the error `E0105`, and `A::L[] = 2;` is `E0700`. A constant is read through the class
name, so `$a::N` is the error `E0496` and `$a->N` is `E0405`.

A subclass that declares the same constant again keeps its type, or a narrower one. An `int|string`
constant may become `int`. A `string` in place of an `int` is the error `E0833`. An interface may
declare a constant, and a class that implements it may declare it again. In a default method,
`static::NAME` reads the value of the class, or the interface's value when the class has none.

`static::NAME` works for `string`, `int`, `uint`, `bool` and `float` constants. For an `array` or a
`?int` constant it is the error `E0832`. A parameter default cannot use `static::` (`E0831`), and
`self::NAME` works there.

`Class::class`, `self::class` and `parent::class` are fixed when the program compiles.
`static::class` and `$obj::class` are read when the program runs. The result is a `string`, so
`int $n = A::class;` is the error `E0401`. `$v::class` on a `mixed`, a `?A` or an anonymous object is
the error `E0702`. Check the value with `is` first. On a `class<A>` value it is `E0702` too. Write
`$c as string` to get the name.

`foreach` works on an array constant like on any other array.
