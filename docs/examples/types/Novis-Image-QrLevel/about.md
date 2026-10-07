How much of a QR code can be damaged and still be read.

You pass a `QrLevel` to `QrCode::render` as `level`. A QR code stores extra data to repair the parts
a scanner cannot read, for example because of dirt or a logo on top of it. The default is `Medium`.

- `Low` repairs about 7 percent of the code.
- `Medium` repairs about 15 percent.
- `Quartile` repairs about 25 percent.
- `High` repairs about 30 percent.

A higher level needs more squares for the same text, so the code gets bigger. When the text is too
long for the level, `render` throws a `LogicError`.
