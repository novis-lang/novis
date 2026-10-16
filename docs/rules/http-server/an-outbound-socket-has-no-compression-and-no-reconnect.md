An outbound socket offers no `permessage-deflate` in its handshake, and a socket that ends stays ended:
it reports why, and what to do next is the program's.

**There is no compression extension** for two reasons that are not about effort. Context takeover holds
a compression window per socket for the socket's whole life, which is memory held per peer rather than
per message. And inflating a peer's frames is a decompression bomb with no length known in advance,
which is a security question of its own. It can be added later as an option, and nothing in the socket's
surface rules it out.

**There is no reconnect.** A reconnect policy is backoff, jitter, resumption tokens and a replay
question, all of them the application's, and no two APIs answer them the same way.

Compression is reconsidered when measurement on a real gateway shows a chatty peer's bytes dominating
a program's cost.
