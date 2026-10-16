Five kinds of number, each for a different job, and none of them quietly gives a wrong answer.

`bool` is true or false and takes part in no sums at all. `int` is a whole number that can go below
zero; `uint` is one that cannot, and counts twice as high in exchange. `float` is the quick,
slightly inexact fraction every machine has in hardware — right for a weight, a ratio or a
measurement. `decimal` keeps exactly the number that was written down, to twenty-eight places,
which is what money needs.

A whole number that runs out of room stops with an error rather than wrapping round to a small
number or turning into a fraction behind your back. Signed and unsigned whole numbers do not mix in
one sum, and neither do decimals and floats: pick the one the job needs and say so where the value
is named.

**Good to know:** a plain number written in the source takes its kind from where it is put, so the
same `6` is a count in one line and a price in the next.
