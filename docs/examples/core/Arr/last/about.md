Gives you the value of the last entry of an array. It replaces PHP's `end`.

Last means last in the order the entries were put in, not the largest key. When you add an entry to
the end of a list, that entry is the one you get back.

The array is not changed, and nothing inside it moves, so asking twice gives you the same value both
times.

An array with no entries gives you `null`. An array can also hold `null` as a value, so a `null`
answer does not on its own say that the array is empty. `Core\Arr::isEmpty` answers that question.
