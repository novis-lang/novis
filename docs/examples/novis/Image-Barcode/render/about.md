Makes a barcode image: lines of bars that a scanner reads, or a square DataMatrix code.

`Barcode::render` takes a string and a `Barcode::Symbology`, which is the type of barcode. It returns
the barcode as an image file. By default the file is a PNG, and the bars are 80 pixels high.

`Code128` takes any plain ASCII text, such as a tracking number. `Ean13` takes the 13 digits printed on
a product.

The options change the image. `height` is the height of the bars in pixels. `module` is the width of
the thinnest bar in pixels. `margin` is the white border on each side, counted in thin bars. `text: true`
prints the data under the bars. `format` is the file `Format`.

A wrong check digit, or a character the type cannot contain, throws a `LogicError`.

**Good to know:** `render` does not add a check digit. Give all the digits, with the check digit last.

**The examples below** show a shipping label, an EAN-13 with its number, and the errors.
