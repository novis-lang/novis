With `[debug] store` on, a request served in `development` records its method, path, query, headers and
the body bytes the program read, up to `[limits] request_body`, and its response's status, headers and
body, up to the same `[limits] request_body`. Before the record is written, the values of `Authorization`,
`Cookie`, `Set-Cookie` and `Proxy-Authorization` are replaced with `[redacted]` in the request and the
response alike, and so is every body field the program decoded into a `secret` property. In
`production` the record holds the request's outcome, and its trace events and dumps are recorded as in
development, but no header and no body of the request or the response is.

The input is what makes a recorded request comparable with another and replayable, and what
`request-aware-editor` turns into a `Core\Test::request` case, which writes a placeholder where the
record says `[redacted]`. Those four headers are replaced by name, before anything reaches the disk,
because they carry a credential in every application, and a replayed request must not log in as the
person who sent the original. A header's name stays, so a compare shows that one was sent. The
response is what a test made from the record asserts: its status, its key headers and its body, a
JSON body compared structurally, and what the viewer's compare view diffs between two requests.

**The response body is capped at `request_body`.** `max_output` has no default cap, so it cannot bound
a record, and the request body's own cap already exists. A longer response body is cut at the cap with
a marker, and a test made from that record asserts only the status and the headers. The body is
written through to the spool file as the program writes it, so it is never held whole in memory.

**The body is what the program read.** The server streams a body to the program one chunk at a time
and reads nothing the program did not ask for (`crates/nvs-server/src/body.rs`), and recording does not
change that: the record keeps each chunk the program pulled, up to `request_body`, and says how many
bytes the request declared. A body the program never read is recorded as not read. Reading the rest
of the body to record it would change what the server does under the debugger, and a large upload would
be held twice.

**A body decoded into a class keeps its `secret` fields off the disk.** When the handler reads the body
through `Core\Request::jsonAs<X>()` or `postAs<X>()`
(`rule:routing/a-request-body-is-read-from-the-handlers-own-call`), the decoder knows which field lands
in a `secret` property of `X` (`rule:security/secret-qualifier`), and the record replaces that field's
value with `[redacted]` in the stored body. A body the program read any other way is stored as it
arrived, so a password in a development login form read as raw bytes is on disk until the index forgets
it (`rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time`), in a directory only
the service's account can open. The `nvs debug` reference page says so. A `jsonAs` or `postAs` that
throws replaces the fields as well, because the bytes it read are recorded either way. Where those
fields cannot be found, the record keeps no byte of the body and says `withheld`: a body that is not
one JSON document or nests deeper than 128 levels, a multipart form, and a class that writes its own
decoder, each where `X` can reach a `secret` property.

- **Memory:** per recorded request, the bytes the program read, up to `request_body`, held until the
  request's record is written at its end. The response body is written through to the spool, so it
  adds one write's bytes at a time. A request with the gate off holds nothing more.

**Declined:**
- **Recording input or a response body in production.** A production request's body is a customer's
  data, and the spool is a debugging aid. The mode, not a second key, decides it. The outcome, the trace
  events and the dumps are what a production incident needs, and they carry no body.
- **Redacting a body field by its name, such as `password`.** A list of names misses the field an
  application named its own way, and the `secret` property is where the program already says which
  value is a credential.
- **Replacing a credential with a hash.** A hash of a short token is a token, so a dictionary turns
  the record back into the credential.
