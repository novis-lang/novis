Makes a `Cell` from its values: the value, its `Style`, and the parts of a rich text cell.

`new Cell` takes `value`, `style` and `runs`, in that order, and each becomes a read-only property.
The value can be anything a row can contain, such as a string, a number, a date or a `Formula`.
`runs` is optional, and its default is `null`.

**Good to know:** `Cell::of` makes the same `Cell`, and most programs use it. `Cell::rich` makes a
rich text cell and checks each part's style.
