`Core\Test::answerHttp()` gives a fixed reply to the web calls your program makes with
`Core\Http\Client`. You give it a URL, a status such as `200` or `404`, and the body of the reply.

After the first call to `answerHttp`, the program makes no network connections. Every web call gets
its reply from the answers you registered. A call that matches no answer throws a `LogicError`. So a
test never reaches a real service by mistake.

A URL that ends in `*` matches every URL that starts with the text before it. When several answers
match, an exact URL wins, and then the longest prefix wins.

The status must be between `100` and `999`. An answer can have `json` or `body`, but not both.

**The examples below** show a reply with JSON, which answer a URL gets, and a client that handles
the error statuses a service can return.
