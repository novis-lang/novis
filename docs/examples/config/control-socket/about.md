Names the local socket an administrator reaches a running server on.

Everything done to a live server goes through it: reloading the configuration, printing what is
currently in force. There is no network listener, no token and nothing to log into — the socket belongs
to the account the server runs as, and the server refuses to start if any other account on the machine
could write the directory holding it. An address another machine could dial is refused for the same
reason. Writing nothing at all, or `false`, leaves the server with no control surface.

**In plain words:** a service door with no handle on the outside of the building. You have to already be
on the machine to be standing in front of it.

**Good to know:** it exists only where a long-running server does, and moving it takes a restart,
because the socket is created once, when the server starts.
