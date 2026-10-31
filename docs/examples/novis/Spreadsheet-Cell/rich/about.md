Writes one text cell whose parts have different fonts, such as a sentence with one bold word.

Give `Cell::rich` a list of parts. Each part has a `text` and may have a `style`. Put the result in
a row in place of the value. Excel shows the parts one after another, in one cell.

A part's style may set `font`, `size`, `bold`, `italic`, `strike`, `underline` and `color`. Any
other field throws a `LogicError` that names the field and the part. An empty list also throws a
`LogicError`. The cell's fill, border, alignment and format come from its row and its column.

The cell's `value` is `null`, and its `runs` property contains the parts.

**Good to know:** `Reader` reads the cell as the text of all parts together. A CSV file has no
fonts, so it also contains that text.
