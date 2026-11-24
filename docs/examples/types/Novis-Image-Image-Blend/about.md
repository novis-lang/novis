How `withOverlay` mixes the colours of the image on top with the colours of the image below it.

`Image::Blend` is declared in the class `Novis\Image\Image`.

An `Image::Blend` is how a pixel on top mixes with the pixel under it:

- `Normal` puts the top pixel over the lower one. Transparent parts of the top image show the lower
  image.
- `Multiply` makes the result darker. White on top changes nothing.
- `Screen` makes the result lighter. Black on top changes nothing.
- `Overlay` makes dark parts darker and light parts lighter, so the contrast is higher.
- `Darken` keeps the darker of the two values for each of red, green and blue.
- `Lighten` keeps the lighter of the two values for each of red, green and blue.

You pass an `Image::Blend` to `withOverlay` as `blend`. The default is `Normal`. You can also store an
`Image::Blend` and compare it, as the example shows.
