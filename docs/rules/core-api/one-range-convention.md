Every member taking a range takes `(offset, ?length)`, and `Core\Str` and `Core\Arr` share the convention
exactly. A negative offset counts from the end; a negative length stops that many elements from the end; a
`null` length runs to the end.

Stated once and never varied, this is one thing to learn instead of one per member. It is also what lets a
reader move between the string and the array class without re-checking whether a second argument is a
length or an end position — the question `substr` and `array_slice` answer the same way and `str_split`
does not.
