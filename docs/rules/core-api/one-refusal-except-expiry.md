Every way of not being authentic is **one error with one sentence**: text that is not a token, a payload
too short, one flipped bit, a key retired past the end of the ring, a missing signature parameter, two of
them. A distinguishable "wrong key" would say which key of a rotating ring a forgery should be aimed at.

**Expiry is the one distinguishable failure**, and the exception is safe because of the *ordering*: the
signature is checked first and the clock only after, so the expired error is reachable only by someone
already holding a valid signature and discloses nothing they do not have. That buys the distinction an
application actually needs — "this link has expired, request another" against "this link is not valid" —
without a disclosure, which is why the ordering is a rule rather than an implementation detail.

Verification answers nothing and throws, because there are no claims to return and a `bool` is a value a
caller can drop on the floor (`rule:core-api/failure-throws`).

**Designed, not shipped.**
