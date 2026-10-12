Makes a QR code as an SVG document: a text file that a browser shows as a sharp image at any size.

`QrCode::svg` takes a string, such as a link, and returns the QR code as an SVG document. The document
is a string, 256 by 256 pixels, with a white border of 4 squares on every side. You can put it straight
into an HTML page.

The options change the code. `margin` is the width of the white border, in squares. `level` is a
`QrCode::Level`, and sets how much damage a scanner can repair.

Text too long for the `level` throws a `LogicError`.

**Good to know:** `QrCode::svg` has no `size` option. The browser scales the SVG to any size, and the
squares stay sharp.

**The examples below** show a link as an SVG, a QR code inside an HTML page, and the border and level.
