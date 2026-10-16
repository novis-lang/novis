How the pixels of one image are mixed with the pixels of the image below it.

`Image::Blend` is declared in the class `Novis\Image\Image`.

An `Image::Blend` is how a pixel on top mixes with the pixel under it:

- `Normal` puts the top pixel over the lower one. Transparent parts of the top image show the lower
  image.
- `Multiply` makes the result darker. White on top changes nothing.
- `Screen` makes the result lighter. Black on top changes nothing.
- `Overlay` makes dark parts darker and light parts lighter, so the contrast is higher.
- `Darken` keeps the darker of the two values for each channel.
- `Lighten` keeps the lighter of the two values for each channel.

**Good to know:** no method of `Image` takes an `Image::Blend`. You can store an `Image::Blend` and compare it, as
the example shows.
