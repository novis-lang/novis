Certificate verification on an outbound call is strict by default and is relaxed only when two
independent things agree: a `[capabilities.tls]` grant names the host, and the call itself asks. Four
options, four grants:

| Option | Grant | Does |
|---|---|---|
| `tlsCa: string` | `tls.anchors` | trusts exactly the PEM certificates given, for this call, in place of `[http.client.tls] roots` |
| `tlsPin: string\|array<string>` | `tls.pin` | accepts a peer whose SubjectPublicKeyInfo hashes to one of the `sha256//<base64>` values, with no chain |
| `tlsVerifyHost: false` | `tls.any_name` | builds and checks the chain, and skips only the name |
| `tlsVerify: false` | `tls.insecure` | checks neither |

**No grant has a `true` spelling.** Each is a list of hosts, matched as `net.connect` matches them, for
`net.internal`'s reason: what a deployment relaxed stays legible host by host in review, where a boolean
is one line nobody reads again. The grant and the option are both owed — a grant changes nothing about a
call that does not ask, and an option whose host the grant does not list throws before a connection is
made, naming the grant, asked of the request's own configuration snapshot per
`rule:security/capability-question-is-grant-and-scope`. Two halves because each answers a different
party's question: the deployment says *where* this may happen, the code says *here*.

A relaxed session still checks the handshake signature against the key the peer presented, so the peer
does hold that key; what is skipped is only the question of whose key it is. The verifiers plug into
`rustls`'s custom-verifier seam inside `nvs_host::tls`, so `rule:security/one-tls-client` still holds and
a caller still cannot hand in a session — it hands in a policy value the module builds one from, and that
policy is part of the pool key
(`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`), so a connection opened
under `tlsVerify: false` never serves a call that verifies. `tlsMinVersion: "1.3"` needs no grant,
because it can only tighten. And the boot prints every relaxed grant, every start, one line per grant and
host: a weakening nobody is reminded of outlives the incident it was added for.
