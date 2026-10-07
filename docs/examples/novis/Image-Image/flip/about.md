Mirrors an image.

`flip(Axis::Horizontal)` swaps left and right, like a mirror next to the image.
`flip(Axis::Vertical)` swaps top and bottom. The size stays the same, and no pixel changes its
colour.

`flip` returns a new `Image` and does not change the one you called it on. The work happens when
`encode`, `variants` or `raw` runs.

**Good to know:** flipping the same way twice gives the original pixels back.
