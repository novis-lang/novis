Gives you the entries of one window of an array.

A position counts entries from the front and starts at 0, whatever the keys are. `$offset` is where
the window opens, and `$length` is how many entries it holds. A negative offset counts back from the
last entry. A negative length stops that many entries short of the end. A `null` length runs to the
end of the array.

The window never reaches outside the array. An offset before the first entry opens at the first
entry. A window that starts after the last entry holds nothing, so you get an empty array.

The result is a new array, and the original is not changed. The entries of the result are numbered
from 0, and the keys of the original are gone. Set `preserveKeys` to keep them.
