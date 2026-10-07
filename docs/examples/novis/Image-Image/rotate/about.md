Turns an image clockwise by a number of degrees.

A turn by 90, 180 or 270 degrees moves the pixels and loses nothing. A turn by 90 or 270 swaps the
width and the height. Any other angle makes the image larger, so the whole turned picture fits.
`background` is the colour of the new corners. Without it, the corners are transparent. A negative
angle turns the image counter-clockwise.

`rotate` returns a new `Image` and does not change the one you called it on. The work happens when
`encode`, `variants` or `raw` runs.

**Good to know:** `Image::open` already turns a photo upright from the orientation the camera wrote
into it. You only need `rotate` for a turn that the photo does not describe.
