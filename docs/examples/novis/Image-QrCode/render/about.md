Makes a QR code: a square image that a phone camera can read back to text.

`QrCode::render` takes a string, such as a link, and returns the QR code as an image file. By default
the file is a PNG of 256 by 256 pixels, with a white border of 4 squares on every side.

The options change the image. `size` is the width and height in pixels. `margin` is the width of the
border, in squares. `level` is a `QrLevel`, and sets how much damage a scanner can repair. `format`
is the file `Format`.

Text too long for the `level` throws a `LogicError`. A `size` smaller than the code, or too big for
the pixel limit, also throws a `LogicError`.

**Good to know:** a longer text or a higher `level` makes a code with more squares, so it needs a
bigger `size`.

**The examples below** show a link with the default options, setting the size, border and format, and
the two errors.
