Starts a response body that your program sends in parts. `stream` returns a
`Core\Response\Stream`, and you call its `write` method once for each part. The client gets each
part when it is written. Use it for a large export, such as a CSV file with many rows, or for a
long job that reports its progress.

`$contentType` is the content type of the body, for example `text/csv`. It becomes the
`Content-Type` header, so a `tainted` value (a value that came from the visitor) does not compile.
A value that is empty or contains a line break throws a `LogicError`.

The status and the headers are sent when you call `stream`, so set them first. The body ends when
the program ends. A response has one body, so using `stream` and `echo` in one response does not
compile.

The examples show a CSV download, the progress of a long job, and an export sent one page at a
time.
