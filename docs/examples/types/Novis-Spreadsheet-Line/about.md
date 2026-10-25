The kind of line that one side of a cell border has.

You set a border with the style field `border` of `Style::of`. A string such as `'thin'` sets the same
line on all four sides. A shape such as `{ top: 'thin', bottom: 'double' }` sets each side alone, and a
side you leave out has no line. `Style::of` turns each string into a `Line` case:

- `'thin'` is `Thin`, the line Excel uses for most tables.
- `'medium'` is `Medium`, and `'thick'` is `Thick`.
- `'dashed'` is `Dashed`, and `'dotted'` is `Dotted`.
- `'double'` is `Double`, two thin lines. It is often used above a total.

Any other string throws a `LogicError`. The field `borderColor` sets the color of the lines.
