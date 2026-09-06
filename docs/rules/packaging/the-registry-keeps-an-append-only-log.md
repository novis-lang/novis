The registry publishes an **append-only transparency log** of `name → version → digest`: a Merkle
tree with signed checkpoints. Before a client uses an artifact it verifies the artifact's inclusion
in the log and the log's consistency with the newest checkpoint the client has seen. An artifact
absent from the log is refused; a checkpoint inconsistent with a previously seen one is refused and
reported as a registry fork rather than as a network error.

A registry that serves one client different bytes than another must therefore either fork the log —
detected by the next client that checks consistency — or produce a signed statement contradicting
one it already made. Compromising the registry stops being silent, which is the property that
matters; the lockfile (`rule:packaging/the-lockfile-holds-every-digest`) then holds the digest so a
client need not trust the registry twice.

The log is also why a registry, rather than git URLs alone, is the transitive source: retraction,
the advisory feed and `nvs audit` all need a namespace with an authority behind it that cannot
quietly rewrite what it said.
