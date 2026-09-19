`&&` means "both", `||` means "either", and `!` means "not". Each one reads its operands as a
condition and gives back `true` or `false`, so the answer is always a `bool` and never the value
that decided it.

Whether a value counts as true is PHP's own table. Everything is true except `false`, `null`, zero,
empty text, the one character `"0"`, and an empty array. A `bytes` value is the one row Novis
answers differently: it is false only when it is empty, so a single `0` byte counts as true. An
object, a closure and an enum case are always true.

`&&` and `||` stop as soon as the answer is settled, so the right-hand side runs only when the left
did not decide it. That is what makes `$count != 0 && $total / $count > 10.0` safe to write: the
division only runs once the count is known not to be zero.

`and`, `or` and `xor` are not part of the language, and neither is a logical operator that returns
one of its operands.

**The examples below** take these in turn: a guard that reads a value only once it is known to be
there, the values that count as empty when a form field arrives, and the checks a request runs
before it is allowed through.
