Makes a barcode as an SVG document: a text file that a browser shows as a sharp image at any size.

`Barcode::svg` takes a string and a `Barcode::Symbology`, which is the type of barcode. It returns the
SVG document as a string. You can put this string straight into an HTML page.

The options are the same as for `Barcode::render`, without `format`. `height` is the height of the bars.
`module` is the width of the thinnest bar. `margin` is the white border on each side, counted in thin
bars. `text: true` adds the data under the bars as a `<text>` element.

A wrong check digit throws a `LogicError`. A character the type cannot contain also throws a
`LogicError`.

**Good to know:** the SVG has a `width` and a `height` in pixels. A printed label stays sharp because
the browser draws the bars, not a picture of them.

**The examples below** show a barcode inside an HTML page, an EAN-13 with its number, and the errors.
