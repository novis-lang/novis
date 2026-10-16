Sets the file format that `encode` and `variants` write.

The first argument is a `Format`: `Format::Jpeg`, `Format::Png`, `Format::Webp`, `Format::Gif` or
`Format::Avif`. Novis can read `Format::Jxl`, `Format::Svg` and `Format::Pdf` files, but it cannot
write them. Choosing one of those throws an error when `encode` runs.

The second argument is optional, with these keys:

- `quality` is from 1 to 100, for JPEG, WebP and AVIF. A higher value gives a better picture and a
  larger file. A value below 1 counts as 1, and a value above 100 counts as 100.
- `lossless` writes a WebP that keeps every pixel exactly. Then `quality` is not used.
- `progressive` and `effort` are accepted. They do not change the file.

A JPEG has no transparent pixels, so the alpha of each pixel is removed. A GIF has at most 256
colours.

Without `format`, `encode` writes the format of the file you opened. A canvas from `create` and pixels
from `fromRaw` are written as PNG.

`format` returns a new `Image` and does not change the one you called it on.
