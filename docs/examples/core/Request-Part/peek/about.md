Returns the first bytes of one uploaded file and keeps them for later reads. After `peek`,
`readAll()`, `content()` and `saveTo()` still give the whole file, from the first byte. Use it to check
the start of a file before you save or read it.

`$max` is the largest number of bytes to return. A file shorter than `$max` is returned whole. A
second call with a larger `$max` reads further into the file. The bytes stay in memory until the
`Core\Request::files()` loop moves to the next file. A `$max` larger than the `[limits] memory`
setting of the request throws a `LogicError`. Call `peek` before `readAll()`, `content()` or
`saveTo()` start to read the file. After that, and after the loop moves on, it throws a `LogicError`.

The examples below check the first line of a CSV file, the first bytes of a file in a program's own
format, and a text file for a zero byte.
