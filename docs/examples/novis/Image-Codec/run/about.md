Runs one image pipeline on a source and returns the result: the encoded file, the pixels, or the size alone.

`Codec::run` needs two values. The source is one of three shapes: an encoded file
(`{data, autoOrient, toSrgb}`), a blank canvas (`{width, height, fill}`) or RGBA8 pixels
(`{width, height, pixels}`). The plan is a shape with `steps`, `overlays` and `output`. Each step is a
shape with exactly one key, which names the operation, for example `{resize: {width: $width}}`. The
steps run in the order of the list.

`output` chooses what `run` returns. `Output::Encoded` returns the file. `Output::Raw` returns 16 bytes
of size and then the pixels, four bytes for each pixel. `Output::Size` returns only the 16 bytes of
size: the width and then the height, each as a 64-bit number.

`Codec::run` throws a `ParseError` when the source is not an image, and an error when a step cannot run
or the image has more pixels than the limit.

**Good to know:** most programs do not call `Codec::run` directly. `Novis\Image\Image` builds the source
and the plan for you, and its `encode` and `raw` methods call `run`.
