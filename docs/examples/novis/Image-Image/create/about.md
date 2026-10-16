Starts an image from a blank canvas of one colour.

`$width` and `$height` are the size of the canvas in pixels. `$fill` is its colour, made with
`Color::hex` or `Color::rgba`. Without `$fill`, every pixel is transparent.

The canvas is made when `encode`, `variants` or `raw` runs. A canvas of width or height 0 throws an
error there. So does a canvas with more pixels than the server's limit.

A canvas did not come from a file, so it has no format of its own. Without `format`, `encode`
writes it as PNG.

**Good to know:** a canvas is useful as a placeholder, as a background, and in tests that need an
image without a file.
