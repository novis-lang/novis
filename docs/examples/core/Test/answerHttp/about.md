`Core\Test::answerHttp()` gives a fixed reply to the web calls your program makes with
`Core\Http\Client`. You give it a URL, a status such as `200` or `404`, and the body of the reply.
An answer can have `json` or `body`, but not both.

After the first call to `answerHttp`, the program makes no network connections. A call that matches
no answer throws a `LogicError`. So a test never reaches a real service by mistake.

A URL that ends in `*` matches every URL that starts with the text before it. An exact URL wins, and
then the longest prefix wins.

You can call `answerHttp` in a test and in a command-line program. In a server, it throws a
`LogicError`. This includes requests, connections, scheduled tasks and queued jobs. So test code left
in your program cannot fake a real service. Code that a test starts may still call it, such as a
request sent with `Core\Test::request`.

**The examples below** show a reply with JSON, which answer a URL gets, and a client that handles
error statuses.
