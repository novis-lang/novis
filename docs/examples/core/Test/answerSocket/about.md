`Core\Test::answerSocket()` gives a test a fake WebSocket service. You give it a URL and a list of
messages. A socket your program opens to that URL with `Core\Http\Client::openSocket` receives the
messages in order. After the last message, `receive` returns `null`, because the service has closed
the connection.

A `string` is sent as a text message, and `bytes` as a binary message. The option `protocol` sets
the subprotocol the service chooses, from the ones the program offers. `Core\Test::sentSocket`
returns the messages your program sent.

After the first call, the program makes no network connections. A URL that ends in `*` matches
every URL that starts with the text before it. An exact URL wins, and then the longest prefix wins.

A test and a command-line program can call `answerSocket`. In server code, it throws a
`LogicError`. Server code is a request, a connection, a scheduled task or a queued job. There is one
exception: a request that a test sends with `Core\Test::request` may call it.

**The examples below** show a service that sends messages, which service a URL gets, and a price
feed client.
