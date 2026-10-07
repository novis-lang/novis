The most pixels `Novis\Image` decodes in one image. You set it in `nvs.toml` as the key
`max_pixels` in the block `[image]`.

Before `Novis\Image` decodes an image, it reads the width and height from the file's header. If
width times height is more than `max_pixels`, the call throws a `RuntimeError`, and nothing is
decoded. For an animation, the pixels of every frame count. This protects the server from a
decompression bomb: a small file whose header claims a huge image.

The value is a number of pixels. `K` means 1,024 and `M` means 1,048,576, so the default `"24M"` is
25,165,824 pixels. That is enough for a photo from a phone camera. `false` and `0` are not allowed.

Only the person who runs the server sets this value. A program cannot raise it. A call to
`Image::open` may pass a lower cap with the option `maxPixels`, and a higher value has no effect.

**The example below** decodes a small upload and stops a huge one, with the cap set to `"1K"`.
