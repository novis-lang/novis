Gives you the value of the first entry of an array.

First means first in the order the entries were put in, not the smallest key. A list gives you the
value at position 0. A map gives you the value that was added first.

The array is not changed, and nothing inside it moves, so asking twice gives you the same value both
times.

An array with no entries gives you `null`. An array can also hold `null` as a value, and then the
answer for an empty array and the answer for a first entry holding `null` are the same. Use
`Core\Arr::isEmpty` when you need to tell those two apart.
