Tests whether at least one entry in an array meets a condition that you write. It replaces PHP's
`array_any`.

You pass the array and a small function. Novis calls that function with one entry at a time, and
the function returns `true` or `false`. The first time it returns `true`, `Core\Arr::any` stops
there and the result is `true`. The entries after that one are never read. If no entry returns
`true`, the result is `false`.

Your function may take a second parameter. Novis then passes the key of the entry as well, always
as a string. In a plain list the key is the position, so it is `"0"`, then `"1"`, then `"2"`.

**Good to know:** an empty array gives `false`. There is no entry that can match. Use
`Core\Arr::find` when you need the matching entry itself instead of a yes or no.
