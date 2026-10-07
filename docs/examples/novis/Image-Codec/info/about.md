Reads the format, the size and the orientation of an encoded image from its header. No pixel is decoded.

`Codec::info` returns a shape. `format` is a `Format` case, and `width` and `height` are the size in
pixels as the file stores them. `hasAlpha` is `true` when the file has an alpha channel. `frames` is the
number of frames in the file, which is 1 for an image that is not animated. `orientation` is the EXIF
orientation from 1 to 8. `exif` contains the EXIF fields as text, or is `null` when the file has none.
`hasIcc` is `true` when the file has a colour profile.

`Codec::info` throws a `ParseError` when the bytes are not an image in a format Novis reads.

**Good to know:** `Image::info` is the same function. Because only the header is read, `info` is fast
even for a very large file, so you can check the size of an upload before you decode it.
