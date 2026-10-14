Returns the position of the first byte your program writes. The position is counted from the start
of the whole body. It works on a stream that `Core\Response::stream` started with a `$length`.

A browser can ask for one part of a body with the request header `Range`. It does this to seek in a
video or to resume a download. The server reads that header, and `offset()` is where the part
starts. Without a `Range` header, `offset()` is `0`.

Your program starts writing at this byte, and writes `length()` bytes. A file is read with `seek`
to this position first. Text in memory is cut with `Core\Bytes::slice`.

A stream without a `$length` is always sent whole, so `offset()` throws a `LogicError` there.

The examples show a part of some text, a file read from its offset, and a stream without a length.
