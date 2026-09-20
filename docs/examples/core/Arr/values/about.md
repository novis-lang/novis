Gives you every value of an array, as a list. It replaces PHP's `array_values`.

The values come back in the order they were put in, and the keys are dropped. The list is numbered
from 0, so the first value is at position 0 and the second at position 1.

The list is a fresh array. You can walk it, count it or change it, and the array you asked about
keeps its own keys and entries.

A common use is after `Core\Arr::filter`, which keeps the key of every entry it keeps. The entries
that are left can have gaps in their numbering, and this gives you the same entries numbered from 0
again.

An array with no entries gives you an array with no entries.
