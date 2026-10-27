Pixels are **RGBA8**, straight alpha at the API, premultiplied where the maths needs it and never
visible to the caller. One model, and it retires whole families of gd concepts outright.

Palette images go: there is one truecolor model, a GIF is quantised on encode, and a colour is a
value rather than a palette index. Mode flags go, because there is no ambient state to hold them.
Per-pixel access goes, replaced by exporting the whole buffer once. Drawing primitives go entirely —
a rectangle is a blank canvas composited, text is a real font, and charts and diagrams are SVG on the
client; a shape layer is not planned, and a third-party extension may carry one. The affine matrix
members go, replaced by the four cases a web application reaches for, named: resize, rotate, flip,
crop. Screen and window capture go, a screen not being a request's business. Format detection by
positional array and integer constants goes, replaced by a typed header shape (`info`) and a format
enum whose MIME type and extension `Image::mime` and `Image::extension` give
(`rule:core-classes/image-pipeline`). A resource to destroy goes, because an image is a value.

What this costs is that a program doing genuine raster drawing has no home here. That is deliberate:
the component exists to transform images a web application receives and produces, and every concept
above was priced against that.

What a gd program becomes, one row per concept:

| gd / exif | Novis |
|---|---|
| `imagecreatefrom*`, `imagecreatefromstring` | `Image::open`, the format detected from the bytes |
| `imagejpeg`, `imagepng`, `imagegif`, `imagewebp`, `imageavif` | `format(...)` then `encode()`; a path is written by the program from the result |
| `imagecopyresampled`, `imagecopyresized`, `imagescale`, `imagecrop`, `imagecropauto`, `imagerotate`, `imageflip`, `imagecopy`, `imagecopymerge` | `resize`, `crop`, `trim`, `rotate`, `flip`, `composite` |
| `imagefilter`, `imageconvolution`, `imagegammacorrect` | `sharpen`, `blur`, `grayscale`, `brightness`, `contrast`, `gamma`, `tint` |
| `imagettftext`, `imagefttext`, `imagettfbbox`, `imageftbbox` | `text`, `measureText` |
| `getimagesize*`, `imagesx`, `imagesy`, `exif_read_data`, `exif_imagetype`, `exif_thumbnail` | `Image::info`; a thumbnail is `resize` |
| `imagesetpixel`, `imagecolorat` | `raw()` and `fromRaw()`, once, the whole buffer |
| palettes, mode flags, drawing primitives, affine matrices, screen capture, gd's own formats | dropped, with the replacement above where one exists |

**Partly on disk.** The component works on RGBA8 frames throughout, and a pixel source and the `raw`
output are RGBA8 rows (`extensions/image/src/lib.rs`). `Image::info` returns `exif` as the EXIF
block's fields of a JPEG, PNG, WebP, AVIF or JPEG XL input, the image's own IFDs without the
MakerNote, and `null` for a file with none (`extensions/image/src/info.rs`,
`crates/nvs-ext/tests/image_decode.rs`).
