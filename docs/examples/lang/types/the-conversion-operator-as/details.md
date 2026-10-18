`as` can be written more than once: `$m as int as string` converts to `int` and then to `string`.

A conversion that fails while the program runs throws one of two errors. A number that does not fit
throws an `ArithmeticError`. These conversions throw it:

- a `float` or a `decimal` with a fraction to `int`
- a negative number to `uint`
- an `int` above 2^53 to `float`

Every other failure throws a `RuntimeError`.

Text converts to a number only when the whole text is one number. `" 42"`, `"42 "`, `""`, `"0x1A"`,
`"1_000"` and `"12.0"` all throw when converted to `int`. `"+5" as int` is `5`. `"1e3" as float` is
`1000`, and `".5"` and `"5."` are also valid `float` values. `"inf"` and `"NAN"` throw.

A `float` converts to `decimal` at the value it prints as. `0.1 as decimal` is `0.1`, not a long
binary fraction. A string converts to `decimal` the same way: `"1e3" as decimal` is `1000`.

`true as string` is `"1"`. `false as string` and `null as string` are `""`. A `bool` does not convert
to a number. `$b as int` is the error `E0708`, so write `$b ? 1 : 0`. Any value converts to `bool`
by the truthiness table: `"0"`, `""`, `0.0` and `null` are `false`.

`string as bytes` always works. `bytes as string` throws a `RuntimeError` when the bytes are not
valid UTF-8.

An enum case converts to `int` or `uint` and gives its number. `5 as Rank` throws a `RuntimeError`
when no case of `Rank` has the number `5`. An enum case does not convert to `string`.

A conversion between two types that has no rule is the error `E0708`, and the message names both
types. An `array`, `bytes` and an enum case to `string` are three of them.

A `mixed` value converts by the type of the value it contains. Text from a `mixed` value is
tainted: `$value as string` gives a `tainted string`.

`array<T> as array<U>` checks each element. An element must already be a `U`, or be a number that
widens to one, such as an `int` in an `array<float>`. The elements are not converted from text:
`["1", "2"] as array<int>` throws a `RuntimeError`.

`?int as int` throws a `RuntimeError` when the value is `null`. `$object as Foo` throws a
`RuntimeError` when the object is not a `Foo`.

A value converts to a set of allowed values, such as `"read"|"write"`, only when it equals one of
them. Otherwise it throws a `RuntimeError` that lists the allowed values.

`as ?T` is not allowed where `as T` cannot fail. `$i as ?int` on an `int` is the error `E0709`. A
class is not allowed either: `$o as ?Foo` is the error `E0473`, so use `$o is Foo`.

`as class<T>` checks a class name written in the code while the program compiles. That is
`Foo::class` or a string literal such as `'Shop\Dog'`. A name that is not a class is the error
`E0303`, and a class that is not a `T` is `E0708`. This is the same with `as ?class<T>`. Every
other string, including a class constant, is checked while the program runs. `as class<T>` then
throws a `RuntimeError`, and `as ?class<T>` gives `null`.

A class name in a string is always the full name with its namespace. No `namespace` and no `use`
applies to it. Inside `namespace Shop`, `"Dog" as ?class<Animal>` is `null`, and
`"Shop\Dog"` finds the class.

A `class<Dog>` converts to `class<Animal>` without a check when `Dog` extends `Animal`. Back from
`class<Animal>` to `class<Dog>`, the class it contains is checked, and the conversion throws a
`RuntimeError` when that class is not a `Dog`.
