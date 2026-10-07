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

**Not shipped.** The component's `run` applies the orientation of a JPEG, PNG or WebP input and
re-encodes JPEG, PNG, WebP, GIF and AVIF with no metadata unless a `metadata` step keeps the EXIF
block, which an AVIF never carries (`extensions/image/src/decode.rs`, `extensions/image/src/encode.rs`,
`crates/nvs-ext/tests/image_pipeline.rs`). AVIF and JPEG XL inputs are not oriented, the ICC
conversion does not exist, and the `Novis\Image` builder does not exist.
