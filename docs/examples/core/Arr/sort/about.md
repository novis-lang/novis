Returns the values of an array in order, from the smallest to the largest.

The keys are renumbered from `0`, so the result is a plain list. Write `{preserveKeys: true}` to keep
every value under its own key. Write `{order: Core\Order::Desc}` for the largest value first. Values
that count as equal keep the order they were added in.

Two numbers are compared as numbers. Two strings are compared character by character, so `"10"` comes
before `"9"`. A number and a string have no order between them, and an array holding both throws an
error. Write `{by: ...}` with a function that returns the value to sort on. Write `{comparator: ...}`
with a function of two values for any other order. If the function in `by` or `comparator` throws
an error, the sort stops and throws the same error.

**The examples below** show the values in order and the largest first, a sort by something you
compute, and a table sorted with its keys kept.

related: Core\Arr::sortByKey, Core\Arr::reverse
