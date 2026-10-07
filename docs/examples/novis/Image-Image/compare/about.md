Compares two images of the same size and returns a `Diff` with the result.

`Image::compare` takes two encoded images, or two `Image` values, or one of each. The `Diff` it
returns has five values. `identical` is `true` when every pixel is the same. `differingPixels` is
the number of pixels that are not the same. `maxDelta` is the largest difference in one colour
channel, from 0 to 255. `ssim` is a score from 0.0 to 1.0 for how similar the two images look, and
it is `1.0` for identical images.

A pixel counts as different only when its difference is above `tolerance`, which is 0 by default.
With `render: true`, `diff` is a PNG file that shows the differing pixels in red. Otherwise `diff`
is `null`. Two images of different sizes throw a `LogicError`.

**Good to know:** in a test, check a number of the `Diff`. For example, test that `ssim` is at least
0.99, so a small change from a new image encoder does not fail the test.

**The examples below** show a test that checks `ssim`, a tolerance for small changes, and the error
for two sizes.
