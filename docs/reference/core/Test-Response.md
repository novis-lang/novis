---
summary: what `Core\Test::request` returns: the status, the headers, the cookies, the body and the error of one in-process request
keywords: Core\Test::request, in-process request, HTTP test, functional test, route table, status, body, header, headers, cookies, json, jsonAs, error, response, no socket
---

`Core\Test\Response` is the value that `Core\Test::request(...)` returns. It contains what the program
under test sent back for one test request. `status()` returns the status code. `header()` and
`headers()` return the headers, and `cookies()` returns the cookies the program set. `body()`,
`json()` and `jsonAs<T>()` return the body. The class has no constructor. You get a response only by
making a request.

The methods work like the ones on `Core\Request`, but they read the response. `header` matches a name
in any case. It joins repeated lines with `, `. `headers` returns an array that maps each lower-case
name to a list of lines. `json` and `jsonAs` take the same `{maxDepth?}` option and throw the same
errors.

The headers are the ones the program sends to a visitor. They include its `Content-Type`, which is
HTML for a page that only used `echo`. They also include a redirect's `Location`, the `Set-Cookie` of
each `addCookie`, and every `setHeader`. The headers that the server adds itself are not included.
`header("Set-Cookie")` throws a `LogicError`, because two cookies joined together are not a valid
cookie. Use `cookies()` to read them by name.

The request runs in the same process. There is no socket and no port. The program's own entry point
runs in a separate isolate (a separate memory space) with the route already matched. So
`Core\Request::route()` in the program returns the same route as for a real request. The real handlers
answer the request, not a mock.

The status is the one the program set, or `200` if it set none. The body is everything the program
wrote. If the program throws an error that it does not catch, the status is `500`. The body is empty,
and there are no headers from the program. The server sends the same response for a failed request.
`error()` returns that error as `Class: message`, so a test can check why the request failed. For a
request that did not fail, `error()` returns `null`.

```nvs skip
#[Test]
public function itReturnsTheUser(): void {
    var $rs = Core\Test::request(Core\Http\Method::Get, "/users/1");

    Core\Test::assertEquals($rs->status(), 200);
    Core\Test::assertEquals($rs->body(), "ada");
}
```

**A request cannot be made from inside a request.** The program that answers a test request is the
same program that sent it. A second request would make it answer its own request forever. So the call
throws a `RuntimeError` that says why. A call outside `nvs test` or `nvs run` throws the same error,
because there is no program under test to answer it.
