Reads the format, the size and other facts about an encoded image, without decoding its pixels.

You can pass `bytes`, an open `Core\IO\File` or an uploaded `Core\Request\Part`. A file keeps its
position, and `saveTo` still saves the whole upload afterwards. The result has these keys:

- `format` is the `Format` of the file, for example `Format::Png`.
- `width` and `height` are the size in pixels, as stored in the file.
- `hasAlpha` is `true` when the image can have transparent pixels.
- `frames` is the number of frames. An animated image has more than one.
- `orientation` is the EXIF orientation from 1 to 8. The value 1 means the image is stored upright.
- `exif` maps each EXIF tag name, such as `DateTime`, to its value as a string. It is `null` when
  the file has no EXIF.
- `hasIcc` is `true` when the file has its own colour profile.

Bytes that are not an image throw an error.

**Good to know:** use `info` to reject an upload that is too large before you decode it.
`Image::mime` and `Image::extension` turn the `format` into a MIME type and a file extension.
