An outbound request carries at most one body, and which of four flat keys in `Core\Http\Options` it was
written under is what says how it is sent: `json` encodes the value as `application/json`, `form` encodes
a map as `application/x-www-form-urlencoded`, `body` sends the octets as given under an optional
`contentType`, and `multipart` sends a map as `multipart/form-data`. Four keys rather than one with a
mode beside it, because a mode string is refused (`rule:core-api/no-mode-strings`) and sniffing cannot
tell a form from a JSON object by looking at an `array<string, string>`.

**Two body keys, a body on `get` or `head`, and `contentType` without `body` are compile-time
diagnostics**, each naming the key. Both halves of the question are in front of the checker: the bag is a
compile-time-constant anonymous object (`rule:core-api/shape-rules` R2) and the verb is the member's own
name, which is exactly why the missing idempotency key is a diagnostic too
(`rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`). Where the verb is dynamic, the same
two checks throw before the first attempt.

A body position admits `tainted`, because posting what a user sent is ordinary and the qualifier is a
question about sinks a request body does not have; and it admits `secret`, as a header already does
(`rule:security/secret-sinks-refuse`'s outbound exemption), which is why a `json` value is walked with
that exemption applied rather than handed to `Core\Json::encode`. `Core\Http\Part` is how a body is sent
without being held: `Part::file` streams from disk under `fs.read` with `Content-Length` from the file's
size. `Content-Length` is always written — every body's size is known before the first byte — so there is
no chunked request body. The body is built once per call and resent unchanged on every attempt, and a
file part is re-read per attempt.
