The local socket an administrator uses to control a running server.

Commands such as `nvs ctl reload` and `nvs ctl status` use it. There is no network port, no password
and nothing to log in to. The socket belongs to the account the server runs as. The server does not
start if another account on the machine can write to the directory that contains it. An address that
another machine can connect to is not allowed. With nothing set, or with `false`, the server has no
control socket.

**In plain words:** a service door with no handle on the outside of the building. You must already
be on the machine to use it.

**Good to know:** only a long-running server has one. If you change the path in the file, the server
creates the new socket before it closes the old one.
