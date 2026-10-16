`Core\Request::files(): Iterable<Part>` yields a request's uploaded parts over `rule:iteration/two-interfaces`'s protocol as they arrive off the connection, one part at a time. It is single-pass, and consuming it is exclusive with `body()` and `bodyStream()` on the same request. There is no buffered sibling and no other member that answers a part: a `post()` never carries a file field, and a body cannot be walked as parts.

The buffered array is absent on purpose. Choosing between a buffered and a streaming accessor needs a per-part size, and that size is not knowable until the part has been consumed — `Content-Length` describes the whole body, and a client's claim about either is only a claim. An API whose safe path is selected from a fact the caller cannot see is a defect independently of what the unsafe path costs, so the choice is not offered.

**Advancing past an unconsumed part drains it.** Skipping an upload the application does not recognise is simply not touching it; the drained bytes are still charged against `upload_total` (`rule:http-server/request-body-and-upload-total-are-two-caps`). Peak memory for an upload of any size is therefore one chunk per in-flight request, plus whatever a call site explicitly asked to hold through `rule:http-server/a-part-is-consumed-in-one-of-three-ways`.

The runtime owns the multipart parser for the same reason. A large browser upload is always
multipart, so a raw `bodyStream()` alone would leave an application two choices: buffer it, or write
the multipart framing itself, with boundary detection across chunk edges and per-part headers. A
parser differential is a security property, so that framing is not an application's business.

**Memory:** one wire chunk per in-flight request while a part is read, whatever the size of the
upload. A buffered part costs more than its size: a `bytes` value has its payload inline and cannot
point into another buffer, so a part copied out of an accumulated body holds the body and the copy at
once, and a permitted 2G upload would be a 4G peak.

The cost is usability. Every program that accepts a file writes a loop, which is three lines in the
common case, and one of them is `saveTo`. A slow client still holds a connection for as long as
`body_idle_timeout` allows, and it no longer holds the body.

**Declined:**
- **A buffered `files()` beside the stream.** The choice between the two cannot be made from
  anything the application can observe, as the paragraph above says.
- **Refusing to advance until the application skips a part explicitly.** It turns the ordinary
  "ignore what I do not know" loop into ceremony and prevents no mistake anyone makes.
