Removes the border around an image.

The border is every row and column at the edge that has the colour of the top-left pixel. `trim`
removes these rows and columns and keeps the rest. `threshold` is how far a colour may differ from
the top-left pixel and still count as border. It goes from 0 to 255, and the default is 10. An
image that is all border keeps its full size.

`trim` returns a new `Image` and does not change the one you called it on. The work happens when
`encode`, `variants` or `raw` runs.

**Good to know:** a photo has noise, so its border is rarely one exact colour. A higher `threshold`
removes more of it.
