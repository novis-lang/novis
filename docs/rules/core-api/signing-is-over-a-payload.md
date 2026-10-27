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

**There are three doors onto the one construction.** `Core\Signature` signs and verifies a payload map;
`$uri->sign`/`$uri->verifySignature` sign the form `$uri->compareTo` normalizes, reserving the `_sig`
query parameter for the token; and `Core\Router`'s pair signs a route's path pattern and parameters, so a
signature survives a remount (`rule:core-classes/router-signed-url`). Signing a URI that already carries
`_sig` replaces it rather than nesting, `_sig` is left out of its own input, and a URL carrying two of
them fails rather than picking one.

A signed **relative** reference leaves scheme, host and port uncovered, so its token is valid on any
origin: right for a link into your own application, wrong for anything else. An absolute one is built
from a configured origin, never from a request's `Host`.

A signed URL authenticates its parameters, not its caller. A link embedded in a page is a credential
its reader holds: it stops them editing `{id}`, which closes the insecure-direct-object-reference
class, and it does not stop them calling the endpoint, which is access control's job.

**Memory:** one signature computation per signing call, and one per key tried per verification until
one authenticates, all inside the call and held nowhere between calls. A program that signs nothing
pays nothing. A token is about `4/3 × (payload + 40)` characters, which is the number to hold against
the path length at which proxies and browsers begin to disagree.

**Declined:**
- **Signing the URL's rendered text.** It is one line and what every framework does, but rendering
  deliberately does not normalize, so two references that are the same URI would produce two
  signatures, and the mismatch would appear only for inputs nobody tested.
- **An option naming which parameters are signed.** It is the configuration point every published
  bypass has gone through, and signing everything present needs no such point.
