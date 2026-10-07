Returns a text as a QR code image: a square image that a phone camera can read back to text.

`Codec::qr` takes the text and a shape with the keys `size`, `margin`, `level` and `format`. A key
that is `null` uses its default. By default the result is a PNG of 256 by 256 pixels. `level` is a
`QrLevel`, which says how much of the code can be damaged and still be read.

`Codec::qr` throws a `LogicError` when the text is too long for the `level`, or when `size` is too
small for the code.

**Good to know:** most programs do not call `Codec::qr` directly. `QrCode::render` calls it, and you
can leave out the keys you do not need.
