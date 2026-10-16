Returns the usual file extension of an image format, without the dot, for example `jpg`.

The argument is a `Format`. The result is a lower-case string you can put at the end of a file name.
`Format::Jpeg` gives `jpg`. Every other case gives its own name in lower case: `png`, `webp`, `gif`,
`avif`, `jxl`, `svg` and `pdf`.

To name a file that a user uploaded, read its format with `Image::info` and pass `$info->format` to
`extension`. The extension then matches the bytes of the file. The name the user gave can be wrong,
so it is not used.

`extension` reads no file and never throws an error.
