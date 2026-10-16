Returns a list of the same value repeated, under the keys `0`, `1`, `2` and so on.

You give the number of entries and the value each one holds. A count of `0` returns an empty array.
The keys always start at `0`, so there is no start index. When you want your own keys, use
`Core\Arr::fillKeys` with the list of keys.

A count so large that the array does not fit in memory throws an error. That matters when the count
comes from a request, because nothing else limits it.

**The examples below** show a list of counters, the placeholders of a database query, and a grid
built from two fills.
