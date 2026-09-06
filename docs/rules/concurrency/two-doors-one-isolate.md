`Core\Socket::upgrade` and `Core\Sse::upgrade` open the same isolate through two different
hand-overs, so a request carries **two cells and not one**.

A WebSocket upgrade *takes the socket*: the hand-over is the descriptor itself, it happens after the
request's own future has ended, and only a request the server framed an upgrade for has one to give.
An event stream takes nothing — its response is an ordinary `200 text/event-stream` that must still
be **sent**, and what the isolate writes into is that response's body while the connection future is
still running.

So the event-stream cell is offered to **every** request a server answers, where the upgrade slot is
offered only to an upgradable one. That is the same fail-closed rule read against a different
hand-over rather than a relaxation of it: what still has no cell, and so is still refused, is
everything that is not a served request — a command-line program, a spawned-script child, a test
method.

Each member names the hand-over *it* is missing, and neither report stands for the other. One slot
carrying both would be a slot the connection has to ask the *kind* of before it could use it, which
is a tag standing in for a distinction the types already make.
