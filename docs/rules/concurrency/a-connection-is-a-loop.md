Inside a connection, code is an ordinary loop — `while (var $msg = $conn->receive()) { … }` — with no
handler interface, no event registration and no second control-flow style. Stackful coroutines are
what make the straight-line loop the *simple* implementation rather than a nicer-looking one.

`Core\Socket::current()` answers the connection, and it is the one member that separates a connection
from everything else: an ordinary request, a spawned-script child and a command-line program all
reach the same refusal, because none of them was handed a peer.

`receive()` suspends the coroutine and answers `?Core\Socket\Message`, `null` when the peer closed —
which is the condition the loop above already ends on. A timeout is therefore not an error a program
sees: an idle or expired connection is closed with its own code and `receive()` answers `null`.
`send()` suspends until the frame is buffered and **throws** on the send timeout rather than waiting
forever, because a program that could not tell a delivered frame from an abandoned one has no way to
recover.

A `Core\Socket\Message` cannot be constructed, so the only way to hold one is to have received it.
