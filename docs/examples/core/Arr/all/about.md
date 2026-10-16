Tests whether every entry in an array meets a condition that you write.

You pass the array and a small function. Novis calls that function with one entry at a time, and
the function returns `true` or `false`. If it returns `false` for an entry, `Core\Arr::all` stops
there and the result is `false`. The entries after that one are never read. If every entry returns
`true`, the result is `true`.

Your function may take a second parameter. Novis then passes the key of the entry as well, always
as a string. In a plain list the key is the position, so it is `"0"`, then `"1"`, then `"2"`.

**Good to know:** an empty array gives `true`. There is no entry that can fail, so nothing makes
the result `false`.
