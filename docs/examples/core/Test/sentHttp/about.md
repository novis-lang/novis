`Core\Test::sentHttp()` returns every HTTP request your program sent to a fake web service. You
create the fake service with `Core\Test::answerHttp`. The list is in the order the requests were
sent, and the first request comes first. Each item is a `Core\Test\SentRequest`.

A `SentRequest` has four methods. `method()` returns the HTTP method, such as `Core\Http\Method::Post`.
`url()` returns the whole URL the program used. `header($name)` returns one header, or `null` if the
request did not have it. The name can use upper or lower case letters. `body()` returns the body as
`bytes`. Reading the list does not clear it. Before your program sends anything, the list is empty.

**The examples below** show how to read one request, how the list grows with each request, and a
test of an API client that must send a token.
