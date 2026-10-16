Colours an image with one colour.

`tint` multiplies the red, green and blue value of every pixel by the values of `$color`. White
becomes the tint colour, and black stays black. The alpha of `$color` is how strong the tint is:
`1.0` is the full tint, `0.5` is half, and `0.0` changes nothing. The transparency of the image
does not change.

`tint` returns a new `Image` and does not change the one you called it on. The work happens when
`encode`, `variants` or `raw` runs.

**Good to know:** call `grayscale` first when the result should have only the tint colour.
