How long a finished outbound connection may sit unused before it is closed rather than kept.

A connection kept for the next call is worth having only while the other end still has it too.
Servers, proxies and load balancers all close connections that have gone quiet, and they do it
without telling anybody — so a connection held too long is not a saved handshake, it is a call that
fails on its first byte and has to be made again.

This is the wait that decides it, and it is the second half of a bound the count beside it cannot
state on its own. The count says how many connections a core may hold; this says how long any one of
them may sit there. Under the count alone, a connection the far end retired hours ago is still one
of the ones being held.

Like the count, it belongs to the core rather than to any one request, so a program may read it and
never change it. It ships shorter than the idle timeout of every common proxy, so that this side is
normally the one that closes — the side that cannot lose a request to the race.
