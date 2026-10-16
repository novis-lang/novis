Returns the status code of a reply as an `int`, for example `200` or `404`.

The status code is the number the server sends first to say how the request went. A code from
200 to 299 means that it worked. A code of 400 or more means an error. An error code does not
throw an error in your program: the request still returns a reply, and your program reads
`status()` and decides what to do. `status()` is a method, so you write `$response->status()`,
with brackets.

**Good to know:** the examples use `Core\Test::answerHttp` to give fixed replies, so they run
without a network. They read a status code, handle a page that was not found, and check a list of
links for broken ones.
