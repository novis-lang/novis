There is no ambient session array and no implicit session start. A script calls `Core\Session::start`
— or an equivalent resuming a presented identifier — before reading or writing session state, so
"this request uses sessions" is a line in the source rather than a fact discoverable only by grepping
for a superglobal. A member called before `start` throws naming itself, and the refusal is catchable
at the root like any other.

The shape is fixed here: a class, an explicit start, no ambient array. The mechanics are a feature of
their own — the store is the shared cache tier or the database, the memory tier is refused at boot
because a session held by one process is not a session, there is no lock, and expiry is the store's
own rather than a sweeper's.

What this costs is one line per request that uses sessions. What it buys is that a request that does
not use them pays nothing, which the ambient version could never promise.

**Starting is not storing.** A `start` for a visitor with no session opens an empty one with no
identifier: no row is written and no cookie is sent. The first write that leaves something in it
draws the identifier and sends the cookie, and the end of the request writes the row. So the store
grows with sessions somebody wrote, never with requests served, and a flood of cookieless requests
that only read leaves nothing behind. `regenerate` on such a session does nothing, because no client
holds an identifier for it, and `destroy` has no row to delete.

**A CSRF token counts as a write.** `Core\Csrf::token` binds its token to the identifier the response
sends, so on a session with none it draws one and sends the cookie, and it marks the session changed
even while the record is empty, so the end of the request writes the row and the next request's
`start` finds it. It spends one row per visitor who is shown a token, and `[session] ttl` expires it.

**Declined:**
- **A session that starts on the first `Core\Session` access.** A read that starts a session is a side
  effect at a distance, the kind of hidden step the explicit `start` exists to remove.
