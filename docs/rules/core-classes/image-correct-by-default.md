Three defaults make the common result the correct one.

`open` **applies the EXIF orientation**, so a phone photo is upright without the caller knowing the
tag exists. `open` **converts an embedded ICC profile to sRGB**, so a CMYK or wide-gamut JPEG resizes
to the colours the photographer saw; every pipeline is sRGB afterwards, and every encoder writes sRGB
without embedding a profile. Both are options that can be turned off by a caller who wants the raw
frame or the raw channels.

`encode` **strips metadata** — EXIF, XMP, IPTC, ICC — unless the plan explicitly kept it. Location
data in a re-encoded upload is the leak this default closes; a program that wants the tags reads them
from the header and stores them where it chooses. A kept EXIF block whose orientation `open` applied
is written with its orientation tag set to `1`, so a viewer does not turn the image a second time.

A profile that does not parse, or that is neither RGB nor CMYK, is ignored, as a browser ignores it:
the pixels are kept as decoded. A CMYK JPEG is read as Adobe stores it, inverted, whether its scan
is CMYK or YCCK.

An AVIF is oriented by its `irot` and `imir` boxes, which browsers follow, and an EXIF orientation
inside it is not applied a second time. A JPEG XL is oriented by its codestream's own field.

**Partly on disk.** The component's `run` applies the orientation of a JPEG, PNG, WebP, AVIF or
JPEG XL input, converts its RGB profile to sRGB, and a JPEG's CMYK one too, and re-encodes JPEG,
PNG, WebP, GIF and AVIF with no metadata unless a `metadata` step keeps the EXIF block, which an
AVIF never carries (`extensions/image/src/decode.rs`, `extensions/image/src/avif.rs`,
`extensions/image/src/encode.rs`, `crates/nvs-ext/tests/image_pipeline.rs`). The `Novis\Image`
builder's `open` turns both on unless its options say `false`.

**Declined:**
- **Keep metadata by default, strip on request.** It is the least surprising default to someone who
  expects a resize to change nothing else. The surprising outcome it allows is a public thumbnail
  carrying a home address, and a default is the only place that outcome is prevented for every
  program rather than for the careful ones.
