How much of a QR code can be damaged and still be read.

`QrCode::Level` is declared in the class `Novis\Image\QrCode`.

You pass a `QrCode::Level` to `QrCode::render` as `level`. A QR code stores extra data to repair the parts
a scanner cannot read. Dirt or a logo on top of the code can hide such parts. This is called error
correction. The default is `Medium`.

- `Low` repairs about 7 percent of the code.
- `Medium` repairs about 15 percent.
- `Quartile` repairs about 25 percent, which is a quarter of the code.
- `High` repairs about 30 percent.

A higher level needs more squares for the same text, so the code gets bigger. When the text is too
long for the level, `render` throws a `LogicError`.
