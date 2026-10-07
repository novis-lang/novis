Makes the middle tones of an image lighter or darker, and keeps black and white.

`1.0` changes nothing. A value above `1.0` makes the middle tones lighter, and a value below `1.0`
makes them darker. Black stays black and white stays white. This is useful for a photo that is too
dark in the shadows but has a correct sky. Transparency does not change.

`gamma` returns a new `Image` and does not change the one you called it on. The work happens when
`encode`, `variants` or `raw` runs.

**Good to know:** `brightness` changes every value by the same factor, so it can turn light parts
white. `gamma` does not change the lightest and the darkest values.
