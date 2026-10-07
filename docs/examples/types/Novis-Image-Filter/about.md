The filter `resize` uses to calculate the pixels of the new image.

You pass a `Filter` to `resize` as `filter`. The default is `Lanczos3`, which gives the sharpest
result for photos and is the right choice in most programs.

- `Nearest` copies the closest pixel. It is the fastest, and it keeps hard edges, so use it for pixel
  art and icons that you make larger.
- `Bilinear` mixes the four closest pixels. It is fast, and the result is a little soft.
- `CatmullRom` and `Mitchell` mix more pixels. They are sharper than `Bilinear` and softer than
  `Lanczos3`.
- `Lanczos3` mixes the most pixels and keeps the most detail.

The filter changes the pixels, never the size of the result.
