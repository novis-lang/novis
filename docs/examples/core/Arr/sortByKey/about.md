Returns the entries of an array ordered by their keys, with every key kept.

Each value stays with its key, and nothing is renumbered. The order is the order of the characters
in the key, so the key `10` comes before the key `9`.

Write `{ order: Core\Arr::Order::Desc }` for the last key first. Write `{ comparator: ... }` with a function
of two keys for another order, for example a numeric one. It returns a negative number when the
first key comes first, zero for equal keys, and a positive number otherwise. Equal keys keep the
order they were added in.

Write `{ keys: [...] }` to put some keys first, in the order you list them. The other keys follow
in their current order. A listed key that the array does not have is skipped. A call with both
`keys` and `comparator` throws a `LogicError`.

**The examples below** show key order, the last key first, numeric keys, and one key moved to the
front of every row.
