Starts an event stream as the response to the current request, and returns a `Core\Sse` object to
send events on.

An event stream is a response that stays open. The server sends events one at a time, and the client
receives each event as soon as it is sent. The server sends the response headers as soon as you call
`stream()`. The stream ends when the request ends. For events that continue after the request, use
`Core\Sse::upgrade`.

An event stream always has the status `200`, and you cannot change it. If the program calls
`Core\Response::setStatus` before `stream()`, then `stream()` throws a `LogicError`.

**The examples below** send two events, show the error for a status that was set first, and report
each step of a deployment as it finishes.
