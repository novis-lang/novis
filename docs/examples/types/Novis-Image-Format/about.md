An image file format.

`Image::format` sets the format that `encode` writes, and `Codec::info` gives the format of a file
that you read. The cases are `Jpeg`, `Png`, `Webp`, `Gif`, `Avif`, `Jxl`, `Svg` and `Pdf`.

`Image::mime($format)` returns the MIME type of a format, such as `"image/webp"`. Send it in the
`Content-Type` header. `Image::extension($format)` returns the file extension without the dot, such
as `"webp"`.

**Good to know:** `encode` writes `Jpeg`, `Png`, `Webp`, `Gif` and `Avif`. `Jxl`, `Svg` and `Pdf` are
formats that `Codec::info` can name. `encode` throws an error for them.
