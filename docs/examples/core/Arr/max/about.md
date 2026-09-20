Gives you the largest value in an array. It replaces PHP's `max` with an array argument.

The values are compared the way `Core\Arr::sort` orders them, so the largest value and the last
value of the sorted array are always the same entry. Numbers compare as numbers. Texts compare byte
by byte, so `"9"` is larger than `"10"` and a text is never read as a number.

Two values of different kinds have no order between them. An array that holds the number 0 and the
text `"a"` throws an error instead of giving an answer.

When several entries are equally large you get the first of them. The keys are not part of the
question: only the values are compared, and only a value is returned. Use `Core\Arr::keyOf` when you
also need the key of the entry you found. An array with no entries gives you `null`.

The examples show the largest number in a list, the best score together with the name that holds it,
and the width of a column in a printed table.
