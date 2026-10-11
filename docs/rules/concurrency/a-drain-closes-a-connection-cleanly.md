A graceful shutdown and a configuration reload close connections with a defined code after a drain
period, so a client's reconnect logic sees a clean close rather than a reset. There is one mechanism
behind both spellings, because there is one thing a stopping process and a reloading one both need.

The drain period is `[server] drain_timeout`, `"30s"` with nothing configured and never unbounded
(`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`). It bounds **work in progress** — a
request whose head, body or response is moving — and it starts when that connection **first sees**
the drain, not when the drain began, so a connection is given the whole period however late it was
accepted. An event stream that is still writing is bounded the same way by the connection block's
own, shorter period.

**Whatever is idle closes the moment it sees the drain**, on every door: an HTTP connection between
requests, a WebSocket whose program is waiting for a frame from its peer, and an event stream with
nothing to write. Nothing idle has anything to finish, and every client already handles the close —
an HTTP/1.1 client retries on a closed kept-alive connection, a WebSocket peer reads the `going
away` close and reconnects, and an event stream's client comes back after the wait its `retry:`
line gave it. A stop that waited a period out for an idle connection would take the whole period
for a browser's one open connection, or a WebSocket's whole idle wait for a peer that had gone
quiet. What is not cut is a program between one `receive` and the next: its `send` completes on its
own clock, and its next `receive` is where it learns the server is going away. A program that only
sends is work in progress with nothing to finish, so the period is its bound: from the moment the
connection first saw the drain plus the connection block's period, its next `send` closes the
connection with `going away` and throws, and a `Core\Time::sleep` on the connection ends at that
instant at the latest and is woken when the drain begins, so a long sleep does not hide a short
period. So a stop is bounded by the work in flight and never by the connections that happen to be
open.

A connection is made to see it: **beginning the drain wakes every parked wait**, rather than leaving
a connection to notice at its own idle timer — which would compose the drain period with
`keepalive_timeout` and make a stop take the longest wait a deployment configured before the period
even started. A wake is not a close; it makes the task runnable, and the close below is still the
one it takes itself.

The close is the connection's *own*, taken at its next wait rather than reached in from the accept
loop. The socket belongs to the isolate, so closing it from outside would be writing to a descriptor
another task is parked on — and a dropped descriptor gives the peer a reset, which is exactly what a
client cannot tell apart from a network failure.

So a stop is bounded by `drain_timeout` plus the write of whatever response was in flight, which
`write_idle_timeout` already bounds. With nothing configured that is a minute, inside systemd's own
default stop timeout of 90 seconds — the generated unit sets no `TimeoutStopSec` — with room for a
distribution that lowered it. An operator whose requests legitimately run longer raises the directive
rather than discovering a `SIGKILL`. A deployment killed inside the bound means the wake is not
reaching some parked wait, which is a defect and not a tuning question.

- **Memory:** an HTTP connection holds one `bool` for the close it has asked for, and each poll of
  it reads the drain bit once, which the idle poll already paid. A WebSocket connection holds one
  shared flag and one drain registration, so a `receive` already parked is cut by the drain, and each
  readable wait on it swaps that flag once. A stream that registered no cut pays nothing.

**Declined:**
- **A fixed drain constant in the server.** Bounded, but an operator could not raise it for a
  deployment whose requests are legitimately long, and the answer to that is a directive.
- **`keepalive_timeout` as the drain period.** It bounds a connection doing nothing, and the drain
  bounds connections doing something (`rule:http-server/the-server-block-is-boot-class`).
- **A shorter default drain period, so an idle connection costs less.** The period bounds a request
  that is working, and a shorter one cuts a slow response short on every restart.
- **Closing an idle connection, or cancelling its isolate, from the accept loop.** The socket
  belongs to the connection's own task, and a close reached in from outside is the reset this rule
  prevents.
- **A wake alone, with `receive` reading the drain bit again on each retry.** The retry loop runs
  inside the host's `read`, under the WebSocket codec, and neither can be told to look at a bit. A
  wake is only a hint (`rule:concurrency/the-parking-contract`), and treating it as more would break
  every other wait.
- **Polling the drain on a short deadline.** Every WebSocket the process holds would be woken every
  few hundred milliseconds for a question that is almost never yes.
- **`Connection: close` on the last response.** It covers only a response written after the drain
  began, so a connection that had already answered, and one a browser opened and never used, would
  still wait.
