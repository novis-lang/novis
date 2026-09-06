Three defaults make the common result the correct one.

`open` **applies the EXIF orientation**, so a phone photo is upright without the caller knowing the
tag exists. `open` **converts an embedded ICC profile to sRGB**, so a CMYK or wide-gamut JPEG resizes
to the colours the photographer saw; every pipeline is sRGB afterwards, and every encoder writes sRGB
without embedding a profile. Both are options that can be turned off by a caller who wants the raw
frame or the raw channels.

`encode` **strips metadata** — EXIF, XMP, IPTC, ICC — unless the plan explicitly kept it. Location
data in a re-encoded upload is the leak this default closes; a program that wants the tags reads them
from the header and stores them where it chooses.

**Not shipped.** No image component exists in the tree.
