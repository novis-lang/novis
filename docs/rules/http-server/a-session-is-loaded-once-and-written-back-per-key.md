`start` reads the whole session once; `get` decodes one key's value, and `set`, `remove` and `clear`
change the copy in the request's own heap. The session is written back when the program that opened
it ends — after any exit hooks, since a hook is user code that may still write — and immediately for
`regenerate` and `destroy`. **Writing only when the session changed** keeps a read-only request off
the write path: a request that starts a session and reads it makes one round trip, not two. A request
that renders a `Core\Csrf::token` for a loaded session is such a request too, because the token is
never stored (`rule:security/csrf-is-on-by-default`).

**A write-back sends only the keys its request set or removed.** The request keeps each key's value
encoded alone, the set of keys it set or removed since the load, and whether it called `clear`. A
touched key that still has a value is a change and one that has none is a removal; a key the request
only read is never sent, so a stale value it read cannot overwrite a newer one. A key's value is
written whole: an array under one key is one value, and two requests that change it keep the last
write. Two requests that change *different* keys of one session keep both changes.

**Each write-back is one atomic step:** one `EVAL` of the write-back script on the shared store, and
one transaction on the `db` store (`rule:http-server/a-session-store-answers-six-operations`). Two
requests that both write `userId` and `role` leave all of one request's values, never half of each.

There is no lock. Nothing is held between `start` and the write-back, and two requests of one session
never wait for each other. A lock held for the length of a request is a cross-request channel
(`rule:security/closed-doors`), and a request that dies holding one wedges every later request for
that session until it expires. The race that remains is two requests that change one key, and the
last write wins; an application for which that matters has state that is not session state, and
locks, counters and idempotency keys go to the shared tier or the database directly
(`rule:concurrency/cross-request-state-is-explicit`).

**`clear` is a whole-session act, as `destroy` is.** Its write-back deletes every key the store holds,
also one another request wrote after this request loaded. A key set after `clear` in the same request
is written back.

**A session with no keys still exists.** It stays in the store until it expires or is destroyed, so
removing its last key does not sign anybody out.

**The store refuses a write-back that would take a session past the reply ceiling.** On the shared
store `start` reads the whole session as one reply, and the client's reply ceiling bounds it; a
session past that ceiling could never be loaded again until it expired. So the write-back script
counts the session's total size and writes nothing when the new total is over the ceiling, and the
end of the request reports that as a lost write naming the size. The `db` store has no read ceiling.

**A destroyed session stays destroyed.** The request's session knows whether the store has the
session under its id: `start` loaded it, or this request drew the id and has not written it yet. The
write-back `save`s a loaded session and `issue`s a drawn one. A `save` changes only a session that
still exists, so when another request ran `destroy` or `regenerate` on it first, the write-back writes
nothing and this request's changes are dropped without a report. That is the last write winning, with
the sign-out as the last write. `regenerate` `move`s the stored session to a new id in one step, so a
key another request wrote survives it; the changes this request has not sent yet are sent to the new
id by the write-back. When the old session was already gone, `regenerate` sends no cookie and leaves
the request a fresh, empty session with no id, as `start` opens for an absent one, so a sign-in
handler that regenerates a session ended elsewhere does not throw.

A cancelled task is the one end that sends nothing, because the send parks.

- **Memory:** per request that started a session, one encoded value per key and the names of the
  keys it touched, released with the request heap. Nothing accumulates per session served, so it is
  O(in-flight). The shared store holds one small hash per session, and the `db` store one row per key.
- **Latency:** one round trip at `start` and one at the write-back, and one for `regenerate`. Only
  changed keys travel, and each `get` and `set` decodes or encodes one value.

**Declined:**
- **A generation counter on the session.** It would detect a write after a sign-out too, but every
  write would need a read first.
- **A request-long lock.** It closes the lost update exactly, but the failure it brings is a wedged
  session, on the request path of every application whether or not it has concurrent writers.
- **A whole-session write-back.** A request undoes other requests' changes to keys it never touched,
  and can write back a stale value it only read.
- **One store key per session key, with a key listing the others.** `clear`, `move` and `destroy`
  would each touch many keys, which the shared tier cannot do in one step. A hash is the same layout
  as one key.
