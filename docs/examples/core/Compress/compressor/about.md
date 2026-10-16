Starts a compression you feed in pieces, instead of all at once.

You get back a `Core\Compress\Compressor`. Call `add` with each piece of data, as many times as you
need, and then call `finish` once. `finish` returns the compressed bytes of everything you added,
in one block. It is exactly what `Core\Compress::compress` returns for the same pieces joined
together, so how you split them up never changes the result.

You choose the format when you open the stream, and it stays that format until the end.

**Good to know:** a stream ends once. After `finish`, both `add` and `finish` throw a
`RuntimeError`, and you open a new stream for the next frame. Reach for this when your data arrives
in pieces; `Core\Compress::compress` is the simpler call when you already hold all of it.
