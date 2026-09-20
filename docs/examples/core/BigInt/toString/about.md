Returns this number as text: its digits in base 10, with a leading `-` when the number is negative.

`echo` and string interpolation render a `Core\BigInt` through this method, so `echo $number` and
`"{$number}"` print exactly what `toString` returns. You call it yourself when you need the digits as
a `string`: to store them, to send them, or to measure how long the number is.

The text is exact at every size. Nothing is shortened, and no separator is added between the digits,
so `Core\BigInt::parse` reads the text back to the number it came from.

**Good to know:** for a base other than 10, use `format`, which writes the same digits in any base
from 2 to 36.
