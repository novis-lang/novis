Reads the format, the size and other facts about an encoded image from the start of the file.

`Image::info` does not decode the pixels, so it is fast even for a large file. It returns a shape.
These are its most useful keys:

- `format` is the `Format` of the file, for example `Format::Png`.
- `width` and `height` are the size in pixels, as stored in the file.
- `hasAlpha` is `true` when the image can have transparent pixels.
- `frames` is the number of frames. An animated GIF has more than one.
- `orientation` is the EXIF orientation from 1 to 8. The value 1 means the image is stored upright.
- `hasIcc` is `true` when the file has its own colour profile.

Bytes that are not an image in a format Novis reads throw an error.

**Good to know:** use `info` to reject an upload that is too large before you decode it.
`Image::mime` and `Image::extension` turn the `format` into a MIME type and a file extension.
