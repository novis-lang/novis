Makes an image lighter or darker.

`brightness` multiplies the red, green and blue value of every pixel by `$amount`. `1.0` changes
nothing. `1.2` makes the image 20% brighter, and `0.5` makes it half as bright. A value can not go
above 255, so very bright parts become white. Transparency does not change.

`brightness` returns a new `Image` and does not change the one you called it on. The work happens
when `encode`, `variants` or `raw` runs.
