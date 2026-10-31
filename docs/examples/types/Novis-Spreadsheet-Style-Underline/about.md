One or two lines under the text of a cell.

You set it with the style field `underline` of `Style::of`. `Style::of` turns the value into a
`Style::Underline` case:

- `true` and `'single'` are `Single`, one line under the text.
- `'double'` is `Double`, two lines under the text.
- `false` gives no underline.

Any other string throws a `LogicError`.
