Mirrors an image.

`flipped(Image::Axis::Horizontal)` swaps left and right, like a mirror next to the image.
`flipped(Image::Axis::Vertical)` swaps top and bottom. The size stays the same, and no pixel changes
its colour.

`flipped` returns a new `Image` and does not change the one you called it on. The work happens when
`encode`, `encodeVariants` or `raw` runs.

**Good to know:** flipping the same way twice gives the original pixels back.
