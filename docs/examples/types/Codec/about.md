Names the compressed format a program reads or writes.

Every `Core\Compress` call says which format it means, and says it as a case rather than as part of a
member's name — PHP ended up with three function names for one algorithm because the format was baked
into the name. There are five cases. `Gzip`, `Zlib` and `Deflate` are the same compressed stream under
three different wrappers: a gzip header and footer, a short zlib header, or nothing around it at all.
`Brotli` and `Zstd` are different algorithms rather than different wrappers, and both are common on the
web today. Every format a browser asks for has a case here, although the `deflate` a browser names is
the zlib wrapper rather than the bare stream.

**Good to know:** a compressed frame carries no label saying which case wrote it, and nothing sniffs
one. A program that stores or sends a frame keeps the format beside it, and reading a frame under the
wrong case is refused rather than guessed at. The examples show the three wrappers over one stream, turn
a `Content-Encoding` header into a case, and pick the format a client said it could read.
