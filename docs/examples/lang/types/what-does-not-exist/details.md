`(int)$x`, `(string)$x`, `(float)$x`, `(bool)$x` and `(array)$x` do not compile (`E0225`). The help
text names the `as` conversion to write, such as `$x as int`.

A call to a free function such as `intval`, `is_int` or `gettype` does not compile (`E0320`). Every
function in Novis is a method of a class, and the built-in ones are under `Core`. `settype` does not
compile either (`E0208`). A variable's type never changes, so you convert into a new variable with
`as`.

`$s[0]` on a string does not compile (`E0482`). `Core\Str::slice($s, $i, 1)` gives one character, and
`Core\Bytes::slice` works with a byte offset.

A string with an arithmetic operator does not compile. `"3" * 2` is `E0716`. `"a" < "b"` is `E0715`,
because `<` does not compare strings. `"3" == 3` is `E0466`, because a `string` is never equal to an
`int`.

`list($a, $b) = $pair;` does not compile (`E0230`). Write `[int $a, int $b] = $pair;`.

A string is never a `callable`, so `callable $c = "strlen";` is `E0418`. A `callable` is an
anonymous function such as `fn (int $n) => $n + 1`, or a method reference such as `Foo::bar`. A
handle, such as an open file or a database connection, is a `Core` object.

A conversion never cuts off a fraction. `3.9 as int` throws an `ArithmeticError`, and `3.9 as ?int`
is `null`.
