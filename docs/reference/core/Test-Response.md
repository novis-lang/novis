---
summary: what `Core\Test::request` answers — the status and the bytes one in-process request produced
keywords: Core\Test::request, in-process request, HTTP test, functional test, route table, status, body, response, no socket
---

`Core\Test\Response` is the value `Core\Test::request(...)` answers: what the program under test wrote
while answering one synthetic request. **It has two members**, `status()` and `body()`, and no
constructor — the only way to obtain one is to make a request.

The request runs in this process. There is no socket and no port: the program's own entry is run as an
isolate with the compiled route table's match already on it, so `Core\Request::route()` inside the
program reads the same match a served request would, and the handler chain that answers is the real one
rather than a mock of it. What comes back is the status the program declared — `200` where it declared
none — and every byte it echoed. A program that throws an error it does not catch answers `500` and an
empty body, which is what the server sends for a failed request.

```nvs skip
#[Test]
public function itReturnsTheUser(): void {
    var $rs = Core\Test::request(Core\Http\Method::Get, "/users/1");

    Core\Test::assertEquals($rs->status(), 200);
    Core\Test::assertEquals($rs->body(), "ada");
}
```

**A request may not be made from inside one.** The program answering an in-process request is the same
program that asked for it, so a second one would answer its own request forever; the call throws
`RuntimeError` instead, naming why. The same throw is what a call outside `nvs test` or `nvs run` gets,
there being no program under test to answer it.
