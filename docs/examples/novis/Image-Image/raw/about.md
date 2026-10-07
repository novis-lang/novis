Runs every step of the image and returns its pixels.

`raw` returns a shape with three keys. `width` and `height` are the size in pixels. `pixels` contains
the image row by row, from the top-left corner. Each pixel is four bytes: red, green, blue and alpha,
each from 0 to 255. So `pixels` has `width` times `height` times 4 bytes.

To read one pixel, take its four bytes with `Core\Bytes::slice` and read them with
`Core\Bytes::unpack` and the format `"C4"`. The pixel at column `x` and row `y` starts at byte
`(y * width + x) * 4`.

`raw` throws the same errors as `encode`. `Image::fromRaw` starts a new image from the shape that
`raw` returns.

**Good to know:** `raw` copies the whole image into your program at once. For work on the whole image,
such as a resize or a blur, the step methods are much faster than a loop over the pixels.
