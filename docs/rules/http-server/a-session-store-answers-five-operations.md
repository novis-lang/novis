A session is a set of keys in a store that can answer *did I issue this id*. A backend answers five
operations and no more:

- `issue` creates a session under an identifier drawn for it, and fails for an id that is live.
- `load` answers every key of the session under an id, or **absent**.
- `save` applies one request's changes — the keys it set, the keys it removed, and whether it called
  `clear` — to a session that is **still live**, refreshes its expiry, and writes nothing for one
  that is gone.
- `move` renames a live session to a new identifier, and answers *done*, *the new one is taken* or
  *the old one is gone*.
- `destroy` deletes the session and answers whether there was one.

`issue` is the only operation that creates a session, so a destroyed session stays destroyed
(`rule:http-server/a-session-is-loaded-once-and-written-back-per-key`). Each operation is one atomic
step, and none needs a read before it, so no request holds anything between two of them.

| Operation | Shared store: one Redis hash per session | `db` store: a head row and one row per key | Replay after a lost reply |
|---|---|---|---|
| `issue` | `EVAL` of the issue script: `0` if the key exists, else `HSET` and `PEXPIRE` | `INSERT` of the head, over an expired one | only if never sent |
| `load` | `HGETALL` | one `SELECT`, a live head `LEFT JOIN` its entries | always |
| `save` | `EVAL` of the write-back script | one transaction: `UPDATE` of a live head's expiry, then the deletes and upserts | always: a second run leaves the same session |
| `move` | `RENAMENX`, which keeps the expiry | one transaction: insert the new head, re-point the entries, delete the old head | only if never sent |
| `destroy` | `DEL` | delete the head and its entries | always |

On the shared store the session's hash lives under `nvs:session-keys:`, so an entry an earlier build
wrote as a string is never read as a hash. Each session key is the field `k:` followed by its bytes,
and the one field `m` holds the total length of the `k:` fields: the write-back script checks the
ceiling against it, and it keeps a session with no keys in the store, since Redis deletes a hash when
its last field goes. A program cannot reach `m`, because every field a program writes is prefixed.
The shared tier is one Redis server, so `RENAMENX` over two keys is allowed; cluster support would
give the session key a hash tag.

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

**Declined:**
- **A signed cookie holding the record itself, with no store.** It costs no round trip, but a store
  that holds nothing cannot answer *did I issue this id*
  (`rule:security/a-session-id-the-store-did-not-issue-is-rejected`). A logged-out session would
  stay valid until its own expiry, and the record's size limit would be a cookie header's.
  `Core\SignedCookie` remains for an application that wants exactly this.
