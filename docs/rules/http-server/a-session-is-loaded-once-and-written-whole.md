`start` reads the record once; `set`, `remove` and `clear` mutate the copy in the request's own
heap; the record is written back when the program that opened it ends — after any exit hooks,
since a hook is user code that may still write — and immediately for `regenerate` and `destroy`.
**Writing only when the record changed** keeps a read-only request off the write path: a request
that starts a session and reads it makes one round trip, not two.

There is no lock. Two requests writing one session concurrently is last-write-wins over the
**whole record**, and this is stated rather than repaired. A lock held for the length of a
request is a cross-request channel (`rule:security/closed-doors`), and a request that dies holding
one wedges every later request for that session until it expires. The cost belongs to a narrow
case — two concurrent requests that write *different keys* of one session lose one of the two
writes — and an application for which that matters has state that is not session state; locks,
counters and idempotency keys go to the shared tier or the database directly
(`rule:concurrency/cross-request-state-is-explicit`).

A cancelled task is the one end that sends nothing, because the send parks.

- **Memory:** one encoded record per request that started a session, released with the request
  heap. Nothing accumulates per session served, so it is O(in-flight).

**Declined:**
- **A per-key write instead of a whole-record one.** It narrows the lost update to the key actually
  contended, but one session becomes many entries, so `clear`, `regenerate` and `destroy` each become
  a multi-key operation, and the shared tier has no multi-key atomic step
  (`rule:core-classes/ratelimit-two-members`).
- **A request-long lock.** It closes the lost update exactly, but the failure it brings is a wedged
  session, on the request path of every application whether or not it has concurrent writers.
