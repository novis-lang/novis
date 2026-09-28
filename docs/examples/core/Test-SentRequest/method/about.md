`method()` returns the HTTP method of one request that your program sent in a test. You get the
request from `Core\Test::sentHttp()`, after `Core\Test::answerHttp` has set up a fake web service.

The result is a case of the `Core\Http\Method` enum. A request sent with `Core\Http\Client::get`
returns `Core\Http\Method::Get`. A request sent with `Core\Http\Client::post` returns
`Core\Http\Method::Post`. A request sent with `Core\Http\Client::request` returns the method you
gave it. You can compare the result with `==` or use it in a `match`.

**The examples below** show how to read the method of one request, the method of each request in a
list, and a test that checks a client removes an item with `DELETE`.
