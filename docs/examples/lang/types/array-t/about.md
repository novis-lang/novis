A list, or a lookup table, whose values are all the same kind of thing.

You say what an array holds where you create it — whole numbers, names, orders — and every value put
into it afterwards has to be one of those. Keys are always text: `$row[3]` and `$row["3"]` reach the
same place, and a loop over the keys is handed text. Entries stay in the order they were added until
something sorts them. An array whose values are themselves arrays is how a table or a grouped report
is built, and it nests as deep as the work needs.

An array is a value rather than a shared box. Copy one into another name, or hand it to a function,
and the other side gets its own; nothing it writes there can turn up in yours.

**Good to know:** reading a key that was never stored stops with an error instead of quietly handing
back nothing, so ask whether a key is there when it is optional.
