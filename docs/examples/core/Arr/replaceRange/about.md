Gives you a copy of an array with one window of entries replaced by other values. It replaces PHP's
`array_splice`, and the array you pass in is never changed.

The window is the one `Core\Arr::slice` returns, so the positions are read the same way. `$offset` is
where the window opens, and a negative offset counts back from the last entry. `$length` is how many
entries the window holds, and a `null` length reaches the end of the array.

A length of 0 puts the new values in front of that position and removes nothing. An offset at the end
of the array adds the values behind the last entry. Leave `$replacement` out and the window is
removed.

The result is a list numbered from 0. The keys of the original are gone, and so are the keys of the
replacement.
