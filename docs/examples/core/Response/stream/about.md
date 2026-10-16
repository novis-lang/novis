Starts a response body that your program sends in parts. `stream` returns a
`Core\Response\Stream`, and you call its `write` method once for each part. Use it for a large
export, a long job that reports its progress, or a video.

`$contentType` becomes the `Content-Type` header, so a `tainted` value does not compile.

The status and the headers are sent when you call `stream`, so set them first. The body ends when
the program ends. Using `stream` and `echo` in one response does not compile.

Give `$length`, the size of the whole body, when you know it. Then a browser can ask for one part
of the body, for example to seek in a video. The stream's `offset()` and `length()` say which bytes
to write. `$etag` is a version name for the body. A browser that resumes a download with an old
name gets the whole body again.

The examples show a CSV download, the progress of a long job, an export sent one page at a time,
and a video that a browser can seek in.
