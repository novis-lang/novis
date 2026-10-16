Makes a `Diff`, the result of comparing two images, from its five values.

`new Diff` takes `identical`, `differingPixels`, `maxDelta`, `ssim` and `diff`, in that order. Each
value becomes a read-only property of the same name. `identical` is `true` when no pixel differs.
`differingPixels` is the number of pixels that differ, and `maxDelta` is the largest difference in one
channel, from 0 to 255. `ssim` is a similarity score from 0 to 1, where 1 means the same image. `diff`
is a PNG with the differing pixels in red, or `null`.

**Good to know:** `Image::compare` makes a `Diff` for you. You write `new Diff` yourself only when you
need one without comparing images, for example in a test of your own code.
