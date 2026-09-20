Finds where one `bytes` value first occurs inside another, and gives you the position as a number of
bytes.

The position is counted from the start of the value, and the first byte is at position 0. When the
bytes you are looking for are not there, the result is `null`. The `from` option starts the search at
a later position, and a negative `from` counts back from the end of the value. A position past
either end of the value is moved to that end, so a search never fails because of the number you gave
it.

The position you get back is the same kind of number `Core\Bytes::slice` takes, so you can cut the
value at the place a search found. Use `Core\Bytes::contains` instead when you only need to know
whether the bytes are there.

**Good to know:** a value that is not found gives you `null` and never `false`, so a match at
position 0 can never be mistaken for "not found".
