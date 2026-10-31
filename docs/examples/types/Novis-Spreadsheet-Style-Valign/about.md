Where the text of a cell is placed from top to bottom.

You set it with the style field `valign` of `Style::of`. `Style::of` turns the string into a
`Style::Valign` case:

- `'top'` is `Top`.
- `'center'` is `Center`.
- `'bottom'` is `Bottom`. Excel uses this when a style does not set `valign`.

Any other string throws a `LogicError`. The place matters when a row is taller than its text, for
example when another cell of the row has `wrap: true`.
