Compares the pixels of two encoded images of the same size.

`Codec::compare` takes two encoded images and a shape with the keys `tolerance` and `render`. It
returns a shape. `identical` is `true` when no pixel differs. `differingPixels` is the number of
pixels whose delta is above `tolerance`, and `tolerance` is 0 when it is `null`. `maxDelta` is the
largest delta, and `ssim` is a similarity score from 0 to 1, where 1 means the same image. `diff` is a
PNG that shows the differing pixels in red when `render` is `true`, and `null` otherwise.

`Codec::compare` throws a `LogicError` when the two images have different sizes, and a `ParseError`
when the bytes are not an image in a format Novis reads.

**Good to know:** most programs do not call `Codec::compare` directly. `Image::compare` calls it,
accepts an `Image` as well as bytes, and returns a `Diff`.
