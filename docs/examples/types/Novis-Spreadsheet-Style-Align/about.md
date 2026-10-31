Where the text of a cell is placed from left to right.

You set it with the style field `align` of `Style::of`. `Style::of` turns the string into a
`Style::Align` case:

- `'left'` is `Left`, and `'right'` is `Right`.
- `'center'` is `Center`. It is common for a header row.
- `'fill'` is `Fill`. Excel repeats the text until the cell is full.
- `'justify'` is `Justify`. Excel makes each line of wrapped text as wide as the cell.

Any other string throws a `LogicError`. Without `align`, Excel puts text on the left and numbers on
the right. The style field `valign` sets the place from top to bottom.
