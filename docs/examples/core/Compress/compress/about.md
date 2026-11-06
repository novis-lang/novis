Makes data smaller by compressing it, in one of five widely used formats.

The data can be a `string`, `bytes`, an open file, an uploaded file, `Core\Request::bodyStream()`,
or a list or loop that gives `bytes` pieces. A file is read from its first byte, one piece at a time.
So you pass a large file directly, and it is never in memory as a whole.

You choose the format with `Core\Compress::Codec`. `Core\Compress::Codec::Gzip` is the one to pick
when the result is sent over a network, because every browser and every server understands it.
`Zlib`, `Deflate`, `Brotli` and `Zstd` are the other four. The result is a block of `bytes`, not
text. `Core\Compress::decompress` reads it back, and you pass it the same format. A format name
that does not exist does not compile.

**Good to know:** compressing a very short value can make it longer, because every format writes a
small header first. For data your program makes one piece at a time, use
`Core\Compress::compressor`.
