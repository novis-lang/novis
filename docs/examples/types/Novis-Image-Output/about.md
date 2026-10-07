What `Codec::run` returns after it has processed an image.

You pass an `Output` to `Codec::run` as `output` in the plan. The result is always bytes.

- `Encoded` returns the encoded image file, in the format the plan sets.
- `Raw` returns the pixels. The first 8 bytes are the width and the next 8 bytes are the height,
  each a big-endian number. Then come 4 bytes for each pixel: red, green, blue and alpha.
- `Size` returns only those first 16 bytes: the width and the height.

**Good to know:** most programs do not use `Output` directly. `encode` uses `Encoded`, `raw` uses
`Raw`, and `Image::measureText` uses `Size`. With `Size`, nothing is encoded, so it is the fastest
of the three.
