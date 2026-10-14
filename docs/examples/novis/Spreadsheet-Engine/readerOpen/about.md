Opens a workbook for reading and returns a `Reading` for it.

`Engine::readerOpen` takes the source and two bounds. The source is the bytes of the file or a
`Core\IO\File` the program opened. The call reads the list of sheets and no rows. Every other
`reader` function takes the `Reading` and the same source again.

A part of the file that is read whole decompresses to at most `maxBytes` bytes. Every part
decompresses to at most `maxRatio` times its compressed size. The server's `[limits]` cap both
bounds, and `null` uses those limits. A part past a bound throws a `ParseError`, and so does a file that is not
a workbook. A source that is not `bytes` or a file throws a `LogicError`.

**Good to know:** most programs do not call `Engine::readerOpen` directly. `Reader::open` calls it
once for a workbook, and reads a CSV file itself.
