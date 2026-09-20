Gives you the entries of an array in runs of the same size. It replaces PHP's `array_chunk`.

`$size` is how many entries each run holds, and the result is a list of those runs. The last run is
short when the number of entries does not divide by the size. An array with no entries gives you no
runs at all. A `$size` of 0 throws a `RuntimeError`, because no run length would ever reach the end
of the array.

Every run is numbered from 0, and the keys of the original are gone. Set `preserveKeys` to keep every
entry under its own key inside its run. The list of runs itself is always numbered from 0, because a
run has no key of its own.

The result is a new array, and the original is not changed.
