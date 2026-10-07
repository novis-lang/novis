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

`cmyk.jpg` is 16 by 16 pixels: the top 8 rows are magenta and yellow ink, the bottom 8 rows cyan
and magenta ink, with no metadata and no colour profile. ImageMagick 7 wrote it with this command,
as a YCCK JPEG with an Adobe marker, the way Photoshop stores CMYK:

```
magick -size 16x8 xc:red -size 16x8 xc:blue -append -colorspace CMYK -strip -quality 100 cmyk.jpg
```

Its colour profile is not a file here. `image_pipeline.rs` writes a small CMYK profile of its own,
so no third-party profile and no redistribution licence is involved, and inserts it into
`cmyk.jpg` as an APP2 segment.

`DancingScript-Regular.ttf` is the font the text tests draw in. It is Dancing Script 1.002, a
Latin TrueType font that kerns through both a `kern` table and `GPOS`, copied unchanged from the
`macroquad` crate's examples. It is under the SIL Open Font License 1.1, which
`DancingScript-OFL.txt` beside it holds with the font's copyright.

Inputs that are broken on purpose, such as a header declaring more pixels than the cap or an EXIF
block with a malformed IFD, are built by the test itself, so they are not files here. So are the
tagged JPEGs: `image_pipeline.rs` inserts an EXIF block it writes, with an orientation of 6 or a
made-up GPS latitude, into `gradient.jpg`.
