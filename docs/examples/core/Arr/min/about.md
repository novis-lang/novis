Gives you the smallest value in an array.

The values are compared the way `Core\Arr::sort` orders them, so the smallest value and the first
value of the sorted array are always the same entry. Numbers compare as numbers. Texts compare byte
by byte, so `"1e2"` is smaller than `"50"` and a text is never read as a number.

Two values of different kinds have no order between them. An array that holds the number 0 and the
text `"a"` throws an error instead of giving an answer. An array that holds only numbers, only
texts, only `true` and `false`, or only objects of one class that can be compared always has an
answer.

The keys are not part of the question. Only the values are compared, and only a value is returned.
An array with no entries gives you `null`.

The examples show the smallest number in a list, the empty array, and the earliest delivery date in
a set of orders.
