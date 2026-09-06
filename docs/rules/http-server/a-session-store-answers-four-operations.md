A session is one record in a store that can answer *did I issue this id*. A backend answers four
operations and no more: `issue` mints an identifier this store has never issued, writes an empty
record under it and answers the id; `load` answers the record under an id, or **absent**; `save`
replaces the record under an id, refreshing its expiry; `destroy` forgets it.

**`load` answering absent is the whole of the strict-id rule.** An id the store did not issue, one
it issued and has since expired, and one an attacker minted are the same answer, and `start`
responds to all three identically: discard it and `issue` a fresh one. There is no separate
`validateId`, because a second question is a second thing that can disagree with the first. A
presented id that is not 22 base64url characters is absent by construction, before any round trip.

The identifier is 128 bits from the CSPRNG `Core\Crypto` draws from, rendered base64url — not a
counter and not a hash of anything the client supplied: an id is a bearer credential for the
length of its life, and the only property it needs is that guessing one is not a strategy. The
record crosses the store boundary as the byte carrier a `Core\Cache` entry does
(`rule:concurrency/a-cached-value-is-copied-across-the-boundary`). `start` is the one member that
talks to the store (`rule:core-api/session-roster`).
