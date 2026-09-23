Whose certificates an outbound `https` call believes, the oldest TLS version it will speak, and the
debugging file it may write its session secrets to.

Novis ships with the Mozilla certificate set and believes it where nothing says otherwise. A
deployment with an authority of its own names that authority beside the shipped set, or names only
its own files where nothing public should be believed at all. The version floor is `1.2` or `1.3`:
the client implements nothing beneath them, so nothing beneath them can be asked for. The secrets
file exists so an operator can read their own traffic in a protocol analyser, and it is refused
outright on a production host — it decrypts everything that host sends, credentials included, for
whoever can read the file.

Only the operator sets these three keys. Every request in the process uses one client, and these
three keys configure it. A program cannot change them for one call or for one request.

A running server takes a change at the next reload, and the next connection uses the new settings.
A connection opened before the reload is not used again. If a file in `roots` has no certificate,
the server does not use the new settings. The reload then names the key as not applied, and the old
settings stay in use.
