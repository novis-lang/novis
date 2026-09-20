Returns the entries of an array ordered by their keys, with every key kept.

Each value stays with its key, and nothing is renumbered. The order is the order of the characters
in the key, so the key `10` comes before the key `9`. It replaces PHP's `ksort`, `krsort` and
`uksort`.

Write `{order: Core\Order::Desc}` for the last key first. Write `{comparator: ...}` with a function
of two keys when you need another order, for example a numeric one. That function returns a
negative number when the first key comes first, zero when the two count as equal, and a positive
number when the second comes first. Keys that count as equal keep the order they were added in.

**The examples below** show the entries in key order, the last key first, and a comparator that
reads the keys as numbers.
