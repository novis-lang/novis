Reads a JSON array one element at a time, so the whole document never has to be in memory.

`Core\Json::decodeEach<T>` reads from an open file, an uploaded file, `Core\Request::bodyStream()`,
`bytes`, or a list or loop that gives `bytes` pieces. A file is read from its first byte. You read
the result once, with a `foreach` loop. Every step gives you one element, checked as
`Core\Json::decodeAs<T>` checks a document. Write `mixed` as `T` to get plain values. Each string in
them is then tainted. With a shape or a `#[Json\Derive]` class, a `string` property is a plain
`string`, and a `tainted string` property is tainted.

The second argument is a JSON Pointer: a path such as `/orders` to an array inside the document. The
default `''` reads the document itself. Set `lines` to `true` to read JSON Lines, with one value on
each line. The pointer must then be `''`.

An error halfway throws a `ParseError` with the byte offset of the error. Your loop already has the
elements before it. Use a database transaction if you need all of them or none of them.

**Good to know:** `decodeEach` does not close the file. Close it yourself.
