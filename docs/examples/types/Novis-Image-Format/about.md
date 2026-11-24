An image file format.

`Image::withFormat` returns a copy of the image with this format set. `encode` writes the file in that
format. `encode` runs every step you added
to the image and returns the finished file. `Codec::info` gives the format of a file that you read.
The cases are `Jpeg`, `Png`, `Webp`, `Gif`, `Avif`, `Jxl`, `Svg` and `Pdf`. `Avif` is the AVIF format,
and `Jxl` is the JPEG XL format.

`Image::mime($format)` returns the MIME type of a format, such as `"image/webp"`. Send it in the
`Content-Type` header. `Image::extension($format)` returns the file extension without the dot, such
as `"webp"`.

**Good to know:** `Image::open` reads all eight formats. `encode` writes only `Jpeg`, `Png`, `Webp`,
`Gif` and `Avif`. `encode` throws an error for `Jxl`, `Svg` and `Pdf`.
