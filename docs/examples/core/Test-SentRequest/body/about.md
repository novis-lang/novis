`body()` returns the body of one request that your program sent in a test. You get the request from
`Core\Test::sentHttp()`, after `Core\Test::answerHttp` has set up a fake web service.

The result is `bytes`, exactly as the program sent them. For a `json` option it is the encoded JSON
document. For a `form` option it is the encoded form text. If the request has no body, as with most
`GET` requests, the result is empty `bytes`. Write `as string` to read the body as text, or pass that
text to `Core\Json::decode` to check each field.

**The examples below** show how to read a JSON body, a form body and an empty body, and a test that
checks the document a shop sends to its payment service.
