Closes a compression stream and returns the compressed bytes of everything you added to it.

You call `finish` once, after your last `add`. It returns one block of `bytes`: your pieces joined in
the order you added them, compressed in the format you chose when you opened the stream. These are
the same bytes `Core\Compress::compress` returns for that data, so the way you split it up never
changes the result.

The stream ends here. It frees the pieces it was holding, and both `add` and `finish` throw a
`RuntimeError` on it from now on. Open a new stream for the next block of data.
