Returns an array that pairs a list of keys with a list of values, one for one.

You give two arrays that hold the same number of entries. The first key goes with the first value,
the second key with the second value, and so on. Both arrays contribute their values, so the keys
they use for themselves are ignored. A key is a whole number or a text, and `1` and `"1"` are one
key. When the same key is given twice, the entry keeps the position of the first one and holds the
last value given for it. Two arrays of different lengths throw a `RuntimeError`. It replaces PHP's
`array_combine`.

**The examples below** show two lists paired into one array, a line of a CSV file read under its
column names, and a lookup that finds a name by its id.
