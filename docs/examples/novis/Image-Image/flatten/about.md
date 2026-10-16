Puts an image on a background of one colour.

Every transparent or half-transparent pixel is mixed with the background colour. The result has no
transparent pixels. The alpha of the background colour is not used.

`flatten` returns a new `Image` and does not change the one you called it on. The work happens when
`encode`, `variants` or `raw` runs.

**Good to know:** JPEG has no transparency. Call `flatten` before you save an image with transparent
parts as `Format::Jpeg`, so you choose the colour of those parts.
