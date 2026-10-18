`and`, `or` and `xor` are the error `E0226`. Write `&&` for `and` and `||` for `or`. For `xor` on two
`bool` values, write `$a != $b`.

`&&`, `||` and `!` always return a `bool`. `5 && 3` is `true`, and `!!$x` gives the truth value of
`$x` as a `bool`. A call that returns `void` is not an operand: `Log::write() && $ok` is the error
`E0718`.

A few values are easy to get wrong. The string `"0.0"` and a string with one space are true. `NAN`
(the float that is not a number) is true. `-0.0`, a `decimal` `0.00` and a `uint` `0` are false.

The right side of `&&` and `||` is narrowed like the branches of an `if`. The narrowing feature
explains the rules. After the whole expression, nothing it tested is kept. So
`bool $ok = $u != null && $u->active;` compiles, but a `$u->active` on the next line is the error
`E0459`.

Warning `W1023` is given when the whole condition of an `if`, a `while`, a `? :` or a `?:` can be
`null`. It is also given for a `?User`, which is false only for `null`. It is not given for an
operand of `&&`, `||` or `!`. So `if ($n && $ok)` with an `?int` `$n` gives no warning, and `null`
and `0` are both false there.
