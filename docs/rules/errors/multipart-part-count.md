`[limits] max_multipart_parts` defaults to **1000** and counts every part, field parts included.
`[limits.hard] max_multipart_parts` defaults to **10000**, and a `[limits]` value above it does not
start. A request may lower or raise its own value up to that ceiling, as with the other `Runtime`
limits.

The part count is one of four bounds on a multipart body, and each bounds a different cost:

- **The part count** bounds bookkeeping. The cost of a part is bookkeeping and a temp file, not
  bytes, so a body well inside every size cap can still hold a million parts, a known
  denial-of-service shape in multipart parsers. 1000 is the number that shape's best-known fix chose.
- **One part's header block** holds at most 16 KiB, which bounds a field name and a filename. A part
  that never ends its headers cannot grow the parse's buffer.
- **`[limits] request_body`** bounds the buffered form fields, their names and their values together.
- **`[limits] upload_total`** bounds the whole body on the wire, file parts included.

A breach is a throw from the call that reads the body, `Core\Request::post()` or the walk over
`Core\Request::files()`, never a status code. A body is read on demand
(`rule:http-server/the-body-is-read-on-demand-under-two-caps`), so no breach is known before a
program reads that far. The message names the directive. The first error a walk meets is kept, and
every later reading call throws it again, so a program that caught the throw never reads the part of
the form that arrived before it.

The part-header search is linear in the body whatever its chunking: each search resumes three bytes
before where the last stopped, so a header block that arrives a byte at a time is not searched again
from its start.

This also widens the attribution rule in the only direction it was missing: **temp files, file
descriptors and disk bytes are charged to a request the same way memory is.**

The cost is that a form with more than 1000 parts needs one configuration line. A ported PHP
application meets the limit exactly where it already met one, because PHP uses the same default.
