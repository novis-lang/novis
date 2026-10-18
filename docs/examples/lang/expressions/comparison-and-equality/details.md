Two values whose types can never be equal are the error `E0466`:

- an `int` and a string, such as `1 == "1"`
- an enum case and an `int`. Compare `$e as int` with the number instead.
- a type that cannot be `null` and `null`, such as `$n == null` for an `int`
- a string and `bytes`
- objects of two classes that are not related

A class and an interface compile, even when the class does not implement the interface. A child
class could implement it.

`int` and `uint` compare exactly, with no conversion: `-1 == 18446744073709551615` is false, and
`-1 < 18446744073709551615` is true. `decimal` ignores trailing zeros: `1.10 == 1.1` is true.
`float` keeps its rounding, so `0.1 + 0.2 == 0.3` is false.

Two strings are equal when they have the same code points (the numbers that make up the text). So
`"é"` written as one code point is not equal to `"e"` followed by a combining accent.

An `array<int>` and an `array<float>` compare element by element, so `[1] == [1.0]` is true.
`["x" => 1, "y" => 2]` is not equal to `["y" => 2, "x" => 1]`, because the keys are in another order.
Two anonymous functions are equal only when they are the same function.

A `?T` value compared with `null` tests whether there is a value. In the branch where `$u != null`
is true, `$u` is a `T`.

Two `mixed` values are compared when the program runs. Values of different types are not equal, so
a `mixed` `1` is not equal to a `mixed` `"1"`. Numbers of different types are equal when the numbers
are the same: a `mixed` `1` is equal to a `mixed` `1.0`.

`<`, `<=`, `>`, `>=` and `<=>` give a compile error on any other pair:

- two strings, two enum cases, two arrays or `null` are the error `E0715`. Order an enum by its
  number with `($a as int) < ($b as int)`.
- two objects of a class that does not implement `Comparable` are the error `E0411`.
- objects of two different classes are the error `E0411`, even when both implement `Comparable`.

A `?int` compiles with `<`. When the value is `null`, the comparison throws a `RuntimeError`, such
as "no ordering for a `null` against a `int`". Two `mixed` strings throw a `RuntimeError` too.

On two objects, `<` calls `compareTo` and compares its result with `0`. `<=>` returns the result of
`compareTo` unchanged. So when `compareTo` returns `-40`, `$a <=> $b` is `-40`. `false <=> true` is
`-1`.
