`Core\Test::answerSocket()` gives a test a fake WebSocket service. You give it a URL and a list of
messages. A socket that your program opens with `Core\Http\Client::openSocket` to that URL connects
to this fake service. It receives the messages in order. After the last message, `receive` returns
`null`, because the service has closed the connection.

A `string` in the list is a text message, and a `bytes` value is a binary message. The option
`protocol` sets the subprotocol that the service chooses. It must be one that the program offers.

After the first call to `answerSocket`, the program makes no network connections. A URL that ends
in `*` matches every URL that starts with the text before it. An exact URL wins, and then the
longest prefix wins. `Core\Test::sentSocket` returns the messages your program sent.

**The examples below** show a service that sends messages, which service a URL gets, and a test of
a client for a live price feed.
