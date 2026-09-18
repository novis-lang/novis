Whose certificates an outbound `https` call believes, the oldest TLS version it will speak, and the
debugging file it may write its session secrets to.

Novis ships with the Mozilla certificate set and believes it where nothing says otherwise. A
deployment with an authority of its own names that authority beside the shipped set, or names only
its own files where nothing public should be believed at all. The version floor is `1.2` or `1.3`:
the client implements nothing beneath them, so nothing beneath them can be asked for. The secrets
file exists so an operator can read their own traffic in a protocol analyser, and it is refused
outright on a production host — it decrypts everything that host sends, credentials included, for
whoever can read the file.

All three are the operator's and nobody else's. Every request in the process reaches the outside
world through one client that these three settle, so there is no per-call and no per-request spelling
for any of them, and a change is in force at the next restart rather than at the next request.
