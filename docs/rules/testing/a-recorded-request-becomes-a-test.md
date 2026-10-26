`nvs debug to-test <trace-id>` reads a recorded request and its response from the debug service's
index and prints a test: one class with one `#[Test]` method that sends the same request through
`Core\Test::request` (`rule:testing/in-process-request`) and asserts what the program answered. The
request is the method, the path and query, the headers and the body the spool recorded. The text goes
through `nvs fmt`'s printer, so it is already formatted. `--write <file>` writes it to a new file and
never replaces one. `--span <id>` picks one request where several share the trace id, which is
otherwise refused. The viewer's button reads the same text from `GET /api/to-test`. The class is
named for the method, the path and the trace id's first eight digits, so two tests made from two
requests to one route compile in one program.

A recorded value goes into the test as a string literal that holds exactly its bytes: single-quoted
where it has no control character, double-quoted with escapes where it has one, and
`Core\Encoding::fromHex` where it has `ESC`, which no literal may hold
(`rule:tooling/an-escape-in-source-text-is-a-compile-error`). A response is the program's own output,
but a request is not, and either can carry text that tries to end a literal; the attack under
`tests/hostile/tools/cli/nvs-debug-to-test/` holds that it stays text.

**The test asserts the response.** The user decided this. The method asserts the status, the key
headers and the body. The key headers are `Content-Type`, and `Location` when the status is a
redirect: the headers the handler chose, not ones that change with every request. A JSON body is read
through the response's `json()` and compared with `assertEquals` against `Core\Json::decode` of the
recorded text, so spacing and escapes do not fail it. Key order does: two arrays are equal only with
their keys in the same order (`rule:expressions/equality-semantics`), and a handler writes its keys in
one order. Any other body is compared as text, or as bytes from hex where no string literal can hold
it. The spool cuts a response body at `[limits] request_body`
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
- **A JSON comparison that ignores key order.** It needs a recursive key sort printed into every test
  or a new assertion, and a handler's key order does not change unless its code does.
