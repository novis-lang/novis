Static serving is one policy in both modes, because a second policy is a second security model. The exact file only, and **never a directory listing**; `index.html` is the sole default document; the MIME type comes from a fixed extension table and an unknown extension is `application/octet-stream`, which the response policy's `nosniff` renders inert. A file a program names with `Core\Response::sendFile` is answered by the same policy as a file a static mount selects.

Freshness is `Cache-Control: no-cache` with a strong `ETag` over `(size, mtime_nanos)` and `If-None-Match` — one validator, exact, and specifically **not** `Last-Modified`, whose one-second granularity serves stale bytes for two edits inside the same second. The tag and the bytes come from one opened file, so a file replaced between a stat and a read cannot pair one file's tag with another file's bytes.

A single `Range` is honoured; a multi-range request, a unit other than `bytes`, or a range the file cannot satisfy is a `416` carrying `Content-Range: bytes */len`, never a silent `200` with the whole body. **`If-Range` is compared strongly against the current `ETag`**, read only when `Range` is present and after `If-None-Match`. A match honours the range; anything else — an old tag, a `W/` tag, an HTTP date, a value that does not parse — ignores the range and answers the whole file with a `200`, including a range that would otherwise be a `416`. A date never matches because this server sends no `Last-Modified`. That is what makes a resumed download safe: a client resuming a file that changed underneath it gets the new file whole, never the tail of one file spliced onto the head of another.

**A file body is streamed from disk, one chunk at a time.** `Content-Length` is exact and stated, so nothing is chunk-encoded, and a file that ends early or fails to read ends the body in an error that cuts the connection: a short body under a stated length would read as a whole file. A `HEAD` reads nothing. There is no configurable `max-age`, no `immutable` and no precompressed-variant lookup: this is a development convenience and a fallback, not a CDN.

**A stream that states its length gets the same decision.** `Core\Response::stream` with a `length` reads `If-None-Match`, `If-Range` and `Range` exactly as a file does, against the `etag` the program gives, and answers `304`, `206` or `416` by the same rules; the decision is one function both callers share. The program writes the window the decision gives (`Core\Response\Stream::offset` and `length`), the count is exact, and a stream that ends before its window is full cuts the connection as a short file does. Without an `etag` neither validator matches; without a `length` the stream is chunked and `Range` is ignored.

**A `.nvs` file is never served as source**, under any dispatch, from any mount.

- **Memory:** one open file handle and one 64 KiB chunk per in-flight file response, plus the HTTP layer's write buffer, whatever the file's size or the range's. The next chunk is read only when that buffer has room, so a slow peer holds the read back. Nothing is cached between requests: O(in-flight), not O(files served).

**Declined:**

- **Reading the whole file or the whole range into memory.** A download of several gigabytes would hold several gigabytes per connection, and the first byte would wait for the last read.
- **A `Last-Modified` validator, or a date in `If-Range`.** One-second granularity serves stale bytes and splices a download edited twice in one second.
- **Serving the range when `If-Range` does not match.** The client would write the tail of a changed file onto the head of the old one.
- **A stream that writes from byte 0 and a server that drops the bytes before the range.** A seek near the end of a large stream would make the program produce everything before it.
