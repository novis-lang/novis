The file permissions of the Unix sockets the server listens on.

A Unix socket is a file, and every account that may write to it may connect to the server. The value
is written as an octal number, the same way `chmod` writes permissions. When the file does not set
it, the value is `0660`: the account the server runs as and the socket's group may connect. To let
another account connect, add it to that group.

This setting has no effect on TCP addresses. The server uses it when it creates a socket, so a new
value takes effect after a restart. A program cannot change it.
