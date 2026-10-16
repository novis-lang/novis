Returns the MIME type of an image format, for example `image/webp`.

The argument is a `Format`. The result is a string you can send as a `Content-Type` header or store
with the file. Each `Format` case has one MIME type:

- `Format::Jpeg` gives `image/jpeg`, `Format::Png` gives `image/png` and `Format::Webp` gives
  `image/webp`.
- `Format::Gif` gives `image/gif`, `Format::Avif` gives `image/avif` and `Format::Jxl` gives
  `image/jxl`.
- `Format::Svg` gives `image/svg+xml`, and `Format::Pdf` gives `application/pdf`.

To find the type of an uploaded file, read its format with `Image::info` and pass `$info->format` to
`mime`. The type then comes from the bytes of the file. A file name can be wrong, so it is not used.

`mime` reads no file and never throws an error.
