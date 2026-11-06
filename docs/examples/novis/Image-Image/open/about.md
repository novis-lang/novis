Starts an image from an encoded file, such as a JPEG, PNG, WebP, GIF, AVIF or JPEG XL upload.

You can pass `bytes`, an open `Core\IO\File` or an uploaded `Core\Request\Part`. `Image::open` only
keeps it and does not read it. The file is read and decoded each time `encode`, `variants` or `raw`
runs, so call them while the file is still open. If it is not an image, that call throws an error.

By default, the image is turned upright. A camera writes an orientation into the file when a photo is
taken sideways, and `open` applies it. The colours are also converted to sRGB, the colour space that
screens and browsers expect. Both steps work the same for every format, AVIF and JPEG XL included.
Pass `autoOrient: false` or `toSrgb: false` to switch either one off.

`maxPixels` sets the largest number of pixels the image may have, counted as width times height. A
larger image throws an error before any pixel is decoded. Without it, the server's own limit applies.

**Good to know:** to read the size and the format without decoding the pixels, use `Image::info`.
