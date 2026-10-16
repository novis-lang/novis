`nvs debug to-test <trace-id>` reads a recorded request and its response from the debug service's
index and prints a test: one class with one `#[Test]` method that sends the same request through
`Core\Test::request` (`rule:testing/in-process-request`) and asserts what the program answered. The
request is the method, the path and query, the headers and the body the spool recorded. The text goes
through `nvs fmt`'s printer, so it is already formatted. `--write <file>` writes it to a new file and
never replaces one. The viewer's button calls the same function.

**The test asserts the response.** The user decided this. The method asserts the status, the key
headers and the body. The key headers are `Content-Type`, and `Location` when the status is a
redirect: the headers the handler chose, not ones that change with every request. A JSON body is read
through the response's `json()` and compared structurally, so key order and spacing do not fail it.
Any other body is compared as bytes. The spool cuts a response body at `[limits] request_body`
(`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`), and a test made from a
cut body asserts the status and the key headers only.

A header the record holds as `[redacted]` is written with the string `'redacted'`, and a comment above
it says to put a test credential there. The test compiles as printed, and a route that checks the
credential fails until the developer replaces it. The credential never reaches the test, because it
never reached the spool. A request whose recorded request body is not the whole body is refused, with
the reason: sending part of a body would test a different request.

The test passes against the program that answered the request, and fails when its handler answers with
another status, another key header or another body.

- **Memory:** development only. One request record is read per command.

**Declined:**
- **Copying the credential into the test.** A test is committed, and a credential in it is published.
- **Asserting every response header.** `Date`, a request id and a cookie change with every request, so
  the test would fail against the program that passed it.
