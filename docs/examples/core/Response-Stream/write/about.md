Sends one part of a response body that `Core\Response::stream` started. The part is a `string` or
`bytes`, and it is sent exactly as it is. An empty part sends nothing and is not an error.

If the client has not finished reading the last part, `write` waits until it has. So only one part
waits in memory at a time, even when the client reads slowly. If the client closes the connection,
or stops reading for longer than the server's send timeout, `write` throws a `RuntimeError`. The
stream is then closed.

The client gets one body. It does not see where one part ends and the next part starts.

The examples show a `string` and `bytes`, parts of different sizes, and a list sent as one JSON
object on each line.
