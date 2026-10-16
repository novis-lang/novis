Searches an array for the first entry that meets a condition you write, and returns the key of that
entry.

You pass the array and a small function. Novis calls that function with one entry at a time, and the
function returns `true` or `false`. The first time it returns `true`, `Core\Arr::findKey` stops
there and the result is the key of that entry. The entries after it are never read. If no entry
meets the condition, the result is `null`.

The key is always a string, whatever you wrote it as. In a plain list the key is the position, so it
is `"0"`, then `"1"`, then `"2"`. Your function may take the key as a second parameter as well.

**Good to know:** use `Core\Arr::find` when you want the entry itself instead of its key.
