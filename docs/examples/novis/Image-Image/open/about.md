Starts an image from the bytes of an encoded file, such as a JPEG, PNG, WebP, GIF or AVIF upload.

`Image::open` only keeps the bytes. It does not read them. The file is decoded when `encode`,
`variants` or `raw` runs. If the bytes are not an image, that call throws an error.

By default, the image is turned upright. A camera writes an orientation into the file when a photo is
taken sideways, and `open` applies it. The colours are also converted to sRGB, the colour space that
screens and browsers expect. Pass `autoOrient: false` or `toSrgb: false` to switch either one off.

`maxPixels` sets the largest number of pixels the image may have, counted as width times height. A
larger image throws an error before any pixel is decoded. Without it, the server's own limit applies.

**Good to know:** to read the size and the format without decoding the pixels, use `Image::info`.
