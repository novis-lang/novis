Makes the difference between light and dark parts larger or smaller.

`contrast` moves the red, green and blue value of every pixel away from middle gray (128) by
`$amount`. `1.0` changes nothing. A value above `1.0` makes dark parts darker and light parts
lighter. A value below `1.0` moves every colour closer to gray. A value stays between 0 and 255.
Transparency does not change.

`contrast` returns a new `Image` and does not change the one you called it on. The work happens when
`encode`, `variants` or `raw` runs.
