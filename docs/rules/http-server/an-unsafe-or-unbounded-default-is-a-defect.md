A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point
a deployment is expected to improve on. The four `[http.*]` blocks exist to make that one sentence
true with nothing written: every response carries the secure header set
(`rule:http-server/secure-headers-with-nothing-written`), CORS is closed
(`rule:http-server/cors-is-closed-until-origins-are-named`), every cookie is `Secure; HttpOnly;
SameSite=Lax` (`rule:http-server/cookies-are-secure-httponly-and-lax`), and no outbound call can
wait forever (`rule:http-server/no-spelling-for-an-unbounded-wait`).

The rule reaches past the client. A `[db.<name>]` pool bound, a socket wait, a terminal prompt and
a queue lease each inherit a finite default and refuse `false` or `0` as a spelling for "no
ceiling", because a wait that never ends is how one slow dependency becomes an outage and the
request-level `wall_time` only bounds the damage after the fact. It also answers what a store
nobody chose means — no store (`rule:http-server/no-session-block-means-no-store`) — since the
safe answer for an absent block is the one that cannot be wrong silently.

What a proxy does earlier and better — request-size caps, per-IP connection limits, flood
limiting — stays the proxy's. Two things do not: how a message is parsed, which is
`rule:errors/ambiguous-input-refused`'s, and how long a connection may idle, which this rule
covers as it covers a client call.
