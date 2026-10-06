Makes data smaller by compressing it, in one of five widely used formats.

You choose the format with `Core\Codec`. `Core\Codec::Gzip` is the one to pick when the result is
sent over a network, because every browser and every server understands it. `Zlib`, `Deflate`,
`Brotli` and `Zstd` are the other four. The result is a block of `bytes`, not text, so keep it in a
`bytes` variable. `Core\Compress::decompress` reads it back, and you pass it the same format.

The format is a value you pass in. If you spell its name wrong, the program does not compile.

**Good to know:** compressing a very short value can make it longer, because every format writes a
small header first.
