Reads a JSON array one element at a time, so the whole document never has to be in memory.

`Core\Json::decodeEach<T>` takes a file that `Core\IO::open` has opened for reading, or a stream of
bytes such as `Core\Request::bodyStream()`. You read the result once, with a `foreach` loop. Every
step gives you one element, checked as `Core\Json::decodeAs<T>` checks a document. Write `mixed` as
`T` to get plain values. Every string is tainted.

The second argument is a JSON Pointer: a path such as `/orders` to an array inside the document. The
default `''` reads the document itself. Values outside that array are checked and skipped. Set
`lines` to `true` to read JSON Lines, with one value on each line. The pointer must then be `''`.

An error halfway throws a `ParseError` with the byte offset of the error. Your loop already has the
elements before it. Use a database transaction if you need all of them or none of them.

**Good to know:** the file stays yours. `decodeEach` opens nothing and closes nothing, so close the
file yourself.
