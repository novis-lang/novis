# Image fixtures

The inputs `crates/nvs-ext/tests/image_decode.rs` and `crates/nvs-ext/tests/image_pipeline.rs` run
through the image component. Each one is a
16 by 12 pixel gradient from red at the top to blue at the bottom, with no metadata, written by
ImageMagick 7 (a free tool) with this command, one run per extension:

```
magick -size 16x12 gradient:red-blue -strip gradient.<ext>
```

`<ext>` is `jpg`, `png`, `webp`, `gif`, `avif` and `jxl`. ImageMagick writes AVIF through libheif and
JPEG XL through libjxl. No file here comes from a photo site, and none carries personal data.

Inputs that are broken on purpose, such as a header declaring more pixels than the cap or an EXIF
block with a malformed IFD, are built by the test itself, so they are not files here. So are the
tagged JPEGs: `image_pipeline.rs` inserts an EXIF block it writes, with an orientation of 6 or a
made-up GPS latitude, into `gradient.jpg`.
