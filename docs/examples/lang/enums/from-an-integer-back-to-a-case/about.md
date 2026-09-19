Turns a number into the case of an enum that carries it.

`$n as Level` gives the case whose number is `$n`. When no case of `Level` carries that number, it
throws a `RuntimeError` whose message names every case of the enum. Write `$n as ?Level` where the
number comes from outside your program: the result is then `null` instead of an error, and your
program chooses what to do about it.

The value on the left is converted to the enum's backing type first, so text and a `float` reach a
case too. `"10" as Level` converts the text to `10` and then looks for the case. Text that is not a
number gives `null` under `as ?Level`, and throws under `as Level`.

**The examples below** convert a number that names a case, then show what happens when no case
carries the number, then read a setting that arrives as text.
