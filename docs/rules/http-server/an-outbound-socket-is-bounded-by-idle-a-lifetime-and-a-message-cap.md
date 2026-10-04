Four bounds cover an outbound socket, and none of them has a spelling for "forever".

| Option | Bounds | Inherits when omitted |
|---|---|---|
| `idle` | the longest silence | `[http.client] idle` |
| `maxDuration` | the socket's whole life | `[http.client] max_duration` |
| `maxMessage` | the largest message after reassembly | `[http.client.socket] max_message` |
| `sendTimeout` | how long a frame may wait to be written | `[http.client.socket] send_timeout` |

`idle` and `maxDuration` are the two bounds and the two directives a streamed reply already reads
(`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`), for the same reason one level
down: an idle check alone never ends a peer that dribbles, and a lifetime alone lets a dead connection sit
until it expires. A message past `maxMessage` closes the socket with `1009`. `ping` is the one knob with an
off position and it is off by default, because a ping is traffic the peer did not ask for; a program that
sets it is choosing to have `idle` end a *dead* peer rather than a quiet one. A peer's ping is always
answered regardless — that is the protocol, not a policy.

Every bound is a `Duration` or a `uint` of bytes, and neither type has an infinite value
(`rule:types/duration`, `rule:core-api/units-are-types`). There is no `null` and no `0` meaning
unbounded, so an outbound socket that waits forever is not something a program can express — the guarantee
comes from the absence of a spelling, exactly as it does for a call
(`rule:http-server/no-spelling-for-an-unbounded-wait`). Expiry throws `TimeoutError`
(`rule:core-api/failure-throws`).

`[http.client.socket]` carries `max_message` and `send_timeout`, both `Runtime`
(`rule:config/three-changeability-classes`), as `[http.client] deadline` is and for its reason: each
bounds one call, and neither is a shared resource one request could spend on another's behalf. Shipped:
`max_message` is four mebibytes and `send_timeout` is thirty seconds — the numbers this process already
applies to the other half of RFC 6455, because one process holding two opinions about the size of one
message is how a program comes to work in one direction and not the other. Neither ships unbounded, which
`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` requires of exactly this kind of wait.
