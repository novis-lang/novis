Makes a `Cell` from its two values: the value and its `Style`.

`new Cell` takes `value` and `style`, in that order, and each becomes a read-only property. The value
can be anything a row can contain, such as a string, a number, a date or a `Formula`.

**Good to know:** `Cell::of` makes the same `Cell`, and most programs use it.
