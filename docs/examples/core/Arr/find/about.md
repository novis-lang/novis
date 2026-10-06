Searches an array for the first entry that meets a condition you write, and returns that entry.

You pass the array and a small function. Novis calls that function with one entry at a time, and the
function returns `true` or `false`. The first time it returns `true`, `Core\Arr::find` stops there
and the result is that entry. The entries after it are never read. If no entry meets the condition,
the result is `null`.

Your function may take a second parameter. Novis then passes the key of the entry as well, always as
a string. In a plain list the key is the position, so it is `"0"`, then `"1"`, then `"2"`.

**Good to know:** the result is also `null` when the array stores `null` as the matching value. Use
`Core\Arr::findKey` when you need to know which entry matched.
