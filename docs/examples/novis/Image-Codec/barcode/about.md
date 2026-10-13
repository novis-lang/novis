Returns a text as a barcode image: lines of bars, or the square DataMatrix code, that a scanner reads
back to text.

`Codec::barcode` takes the text, a `Barcode::Symbology` and a shape with the keys `height`, `module`,
`margin`, `text` and `format`. A key that is `null` uses its default. By default the result is a PNG.
`module` is the width in pixels of the thinnest bar. With `text` set to `true`, the data is printed
under the bars.

`Codec::barcode` throws a `LogicError` when the check digit is wrong, or when the text has a character
the barcode type cannot hold. It never adds a check digit for you.

**Good to know:** most programs do not call `Codec::barcode` directly. `Barcode::render` and
`Barcode::svg` call it, and you can leave out the keys you do not need.
