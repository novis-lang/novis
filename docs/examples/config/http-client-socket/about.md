The two bounds an outbound WebSocket has that no other outbound call does: the largest message it
will put back together, and how long one frame may wait to be written.

A WebSocket stays open and delivers messages that arrive in pieces, so the receiving side has to
hold the pieces until the last one lands. The message cap is how large that may get before the
socket is closed instead — without it, a peer decides how much memory the task holding the socket
uses. The send timeout is the other direction: how long a frame may wait for a peer that has stopped
reading before the send fails.

Neither has a spelling that means "no limit". Both `false` and zero are refused where the
configuration is read, so a socket that waits forever or grows without end is not something this
file can ask for.

Unlike the rest of `[http.client]`, these two are a program's to change. Each bounds the one socket
the program opened, and neither spends anything another request then goes without — so a program
that knows its own peer names its own value at the call instead.
