`header($name)` returns the value of one header of a request that your program sent in a test. You
get the request from `Core\Test::sentHttp()`, after `Core\Test::answerHttp` has set up a fake web
service.

Upper and lower case letters in the name do not matter, the same as in HTTP. `header("Authorization")`
and `header("authorization")` return the same value. If the request has no header with that name,
the result is `null`. A test uses this to check a token, a content type or a language that the
program sent.

**The examples below** show how to read one header, what a missing header returns, and a test that
checks every request of an API client carries its token.
