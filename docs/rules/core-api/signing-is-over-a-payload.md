Signing takes a **structured payload**, never assembled text. The signing member canonicalizes the payload
itself — keys sorted, each value encoded with its type — so there is no assembled string for the signing
side and the verifying side to disagree about, and the lifetime travels *inside* the signed bytes rather
than beside them. The token is URL-safe by construction, so nothing downstream escapes it again.

A URL is a payload the URI class already canonicalizes. Signing a URL signs the normalization its own
equivalence comparison defines — scheme and host folded to lower case, escape digits upper-cased, an escape
spelling an unreserved character decoded, dot segments removed — reached a second time rather than invented
a second time. That is the whole point: every framework's signed-URL bug class comes from a canonical form
used by nothing but the signature, so nothing else exercises it and no other test constrains it. Reusing a
form that is already load-bearing turns a new risk into a second caller of an existing one, and a drift
becomes one bug in one function that the existing corpus catches.

Every component present is covered, so appending any parameter invalidates the signature; there is no
option naming which parameters are signed, because that option is where every framework's bypass has lived.
The fragment is never signed, since it is not sent to the server.

**Two doors of the three are on disk.** `Core\Signature` signs and verifies a payload map, and
`$uri->sign`/`$uri->verifySignature` sign the form `$uri->compareTo` normalizes, reserving the `_sig`
query parameter for the token. `Core\Router`'s pair — the one that signs a route's name and parameters,
so a signature survives a remount — is not registered yet.
