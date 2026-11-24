Makes the edges in an image clearer.

`sharpened` makes the light side of an edge a little lighter and the dark side a little darker.
Areas of one colour do not change. `sigma` is how wide an edge is, in pixels. The default is `1`,
which suits small details. A larger value sharpens wider edges. Transparency does not change.

`sharpened` returns a new `Image` and does not change the one you called it on. The work happens
when `encode`, `encodeVariants` or `raw` runs.

**Good to know:** a smaller image looks soft after `resized`. Call `sharpened` after `resized` to
make a thumbnail look clear again.
