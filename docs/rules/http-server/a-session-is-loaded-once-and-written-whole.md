`start` reads the record once; `set`, `remove` and `clear` mutate the copy in the request's own
heap; the record is written back when the program that opened it ends — after any exit hooks,
since a hook is user code that may still write — and immediately for `regenerate` and `destroy`.
**Writing only when the record changed** keeps a read-only request off the write path: a request
that starts a session and reads it makes one round trip, not two. A request that renders a
`Core\Csrf::token` for a loaded session is such a request too, because the token is never stored
(`rule:security/csrf-is-on-by-default`).

There is no lock. Two requests writing one session concurrently is last-write-wins over the
**whole record**, and this is stated rather than repaired. On the `db` store each operation is one
statement in its own transaction, so nothing is held between `start` and the write-back, and two
requests of one session never wait for each other. A lock held for the length of a
request is a cross-request channel (`rule:security/closed-doors`), and a request that dies holding
one wedges every later request for that session until it expires. The cost belongs to a narrow
case — two concurrent requests that write *different keys* of one session lose one of the two
writes — and an application for which that matters has state that is not session state; locks,
counters and idempotency keys go to the shared tier or the database directly
(`rule:concurrency/cross-request-state-is-explicit`).

**A destroyed session stays destroyed.** The request's session knows whether the store has a row
under its id: `start` loaded one, or this request drew the id and has not written it yet. The
write-back `save`s a loaded session and `issue`s a drawn one. A `save` changes only a row that
still exists, so when another request ran `destroy` or `regenerate` on the session first, the
write-back writes nothing and this request's changes are dropped without a report. That is the
last-write-wins above, with the sign-out as the last write. `regenerate` checks the same thing: it
issues the record under a new id, then destroys the old row, and when that row was already gone it
destroys the new one too, sends no cookie and leaves the request a fresh, empty session with no id,
as `start` opens for an absent one. A sign-in handler that regenerates a session ended elsewhere
does not throw. What it spends: one `bool` per in-flight session, no extra round trip, and one more
delete in the `regenerate` race alone.

A cancelled task is the one end that sends nothing, because the send parks.

- **Memory:** one encoded record per request that started a session, released with the request
  heap. Nothing accumulates per session served, so it is O(in-flight).

**Declined:**
- **A generation counter on the row.** It would detect a write after a sign-out too, but every
  write would need a read first.
- **A per-key write instead of a whole-record one.** It narrows the lost update to the key actually
  contended, but one session becomes many entries, so `clear`, `regenerate` and `destroy` each become
  a multi-key operation, and the shared tier has no multi-key atomic step
  (`rule:core-classes/ratelimit-two-members`).
- **A request-long lock.** It closes the lost update exactly, but the failure it brings is a wedged
  session, on the request path of every application whether or not it has concurrent writers.
