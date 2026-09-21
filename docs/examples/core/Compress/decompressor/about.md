Starts a decompression you feed in pieces, instead of all at once.

You get back a `Core\Compress\Decompressor`. Call `add` with each piece of the compressed data, as
many times as you need, and then call `finish` once. `finish` returns the original bytes. Where you
split the pieces never changes the result, so a piece may end in the middle of anything.

The result is `tainted bytes`. The data came from somewhere else, and unpacking it does not make it
safe to trust, so Novis keeps track of that for you.

The limit works the same way it does for `Core\Compress::decompress`, and it is fixed when you open
the stream. `$maxBytes` is the largest result you accept and `$maxRatio` is the most bytes of result
per byte you fed in. `finish` measures the whole stream against them, not each piece, so ten pieces
cannot get ten times the room. This replaces PHP's `inflate_init` and `inflate_add`.

**Good to know:** a stream ends once. After `finish`, both `add` and `finish` throw a
`RuntimeError`.
