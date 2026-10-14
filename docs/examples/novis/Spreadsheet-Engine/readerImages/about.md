Returns the pictures of one sheet.

`Engine::readerImages` takes a `Reading`, its source and the name of a sheet. Each picture has its
top-left cell in `at`, its bytes in `image`, and its `format`, such as `png`. A linked picture is
stored outside the file. It has empty `image` bytes, and `link` is its target. Reading a linked
picture never fetches it.

A sheet name the workbook does not have throws a `LogicError`. So does a source that is not `bytes`
or a file. The source is the one `Engine::readerOpen` read. Another source can throw a `ParseError`
or give wrong results.

**Good to know:** most programs do not call `Engine::readerImages` directly. `Reader::images`
calls it once, and also finds a sheet by its position.
