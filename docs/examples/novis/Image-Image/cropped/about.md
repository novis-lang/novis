Cuts a box out of an image and keeps only that box.

`x` and `y` are the top-left corner of the box, in pixels from the top-left corner of the image.
`width` and `height` are the size of the box. The box must lie completely inside the image and must
not be empty. Otherwise `encode`, `encodeVariants` or `raw` throws an error.

`cropped` returns a new `Image` and does not change the one you called it on. The work happens when
`encode`, `encodeVariants` or `raw` runs.

**Good to know:** to keep the middle of an image at a fixed shape, `resized` with both `width` and
`height` is often easier. It cuts off the edges for you.
