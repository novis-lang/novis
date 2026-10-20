A program that never reads its request body is still answered. The body is not drained for it: once nothing reads a body, `hyper` reads what one read of the socket already holds and stops. A body that ends there leaves the connection open for the next request. A longer one ends reading on the connection, so the response is the connection's last and a request pipelined behind it is not answered. A `413` for a declared length past `upload_total` is the same case, with nothing of the body read at all.

**The close comes after the response, in two steps.** A socket closed while its peer's bytes are still unread in it is reset, not closed, and a reset can discard a response the peer has received and not yet read. On the loopback a client that writes a 4 MiB body before it reads anything loses its response that way. So once the last response is written, the server sends its FIN first, then reads and discards what the peer still sends until the peer closes its side, the socket fails, or `LINGER` (two seconds, a named constant in `nvs_server::serve`) passes. Under a drain, the drain period's end comes first if it is sooner. A connection that ended in an error — an expired wait, a refused head — is closed at once, and so is one whose task is cancelled. An upgraded connection is not closed at all: its socket belongs to its isolate.

**What is read while closing reaches nothing.** It is not given to a program or to `hyper`, and it is not counted against `upload_total`, so the time bound is the whole bound. The body bound a request has does not grow: a body nobody reads is not delivered past the one read `hyper` makes.

- **Memory:** one 8 KiB scratch buffer of the connection's own stack while it reads, and the connection's coroutine for up to two seconds past its last response. Per connection being closed, so O(in-flight). A peer that closes once it has the response ends the wait at that read.

**Declined:**

- **Draining an unread body before the response.** The client chooses how long the body is, so the server would read as much as `upload_total` allows for a program that wanted none of it, and the response would wait for all of it.
- **A byte bound on the close.** A client that writes its whole body before it reads gets a reset at any bound smaller than its body. The two seconds bound the cost whatever the client sends.
- **A setting for the two seconds.** No deployment has a reason to choose it, and a proxy in front closes its side as soon as it has the response.
