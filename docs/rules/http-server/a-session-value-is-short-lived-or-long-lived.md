**Every session value has one of two lifetimes, and the member that writes it names which.** A
short-lived value, written by `setShortLived` or `setShortLivedSecret`, is the login kind: it ends
when the browser ends its session or the short clock runs out. A long-lived value, written by
`setLongLived` or `setLongLivedSecret`, is the preference kind, such as a consent choice or a cart: it
survives the browser closing and lives for `[session] long_lived`
(`rule:http-server/session-expiry-belongs-to-the-store`). Both live in one record under one identifier
and share one set of keys. The last write of a key sets its lifetime, and `get`, `getSecret` and
`remove` read or delete a key of either lifetime.

**Two cookies keep the two lifetimes apart.** One identifier serves both kinds, so a cookie that
survives the browser closing would carry the login across a restart too. So the short-lived values are
bound to a second cookie:

- The identifier cookie, `[session] cookie`, has `Max-Age = long_lived` while the record has a
  long-lived value, and no `Max-Age` otherwise. It is sent again with a new `Max-Age` when the long
  clock is refreshed.
- The short cookie, named `[session] cookie` followed by `-short`, carries a 128-bit token from the same
  CSPRNG as the identifier. It never has `Max-Age`, so the browser deletes it when its session ends. It
  is drawn at the first short-lived write in a session with no valid token, and drawn again by
  `regenerate` and `clearShortLived`.
- The record keeps the token. At load, the short-lived values are visible only when the presented
  token matches, compared in constant time. A mismatch or a missing token makes them absent, and the
  session counts as changed, so the write-back deletes them. That is one write per browser restart.
- A session opened by `openWithIdentifier` has no browser, so its short-lived values are bound to the
  identifier alone and no token is checked.

Both cookies take every other attribute from `[http.cookies]`
(`rule:http-server/cookies-are-secure-httponly-and-lax`).

**The clears.** Every clear sends a cookie, so all three throw where the head is closed, as
`regenerate` does.

- `clearShortLived` deletes the short-lived values, moves the session to a new identifier with the
  store's `move`, and draws a new short token. It is the logout, and a consent choice survives it.
- `clearLongLived` deletes the long-lived values and moves the session to a new identifier.
- `clearAll` deletes the record, sends both cookies with `Max-Age=0`, and leaves the request a fresh,
  empty session with no identifier. A later write in the same request draws a new identifier, as on a
  first visit.

Across requests `rule:http-server/a-session-is-loaded-once-and-written-back-per-key` holds: a write-back
never brings back a session another request cleared.

**The store layout.** On the shared store each key's hash field is `s:<key>` for a short-lived value and
`l:<key>` for a long-lived one, and the write-back script deletes the other prefix's field when it writes
a key. The fields `t` (the short token) and `x` (the short clock) sit beside `m`. On the `db` store
`nvs_session_entries` has a `long` column, and `nvs_session_heads` has `short_token` and
`short_expires_at`. Every operation stays one command or one transaction. There is no size limit of its
own: the record's bound is the reply ceiling the per-key rule states.

Cost: per request, one round trip at the first use, as with one lifetime. A second cookie of 22
characters on the requests of a session with short-lived values, and one constant-time compare at load.
One write per browser restart. On the store, a record with a long-lived value is kept for `long_lived`.

**Declined:**
- **Two stores behind two classes.** Two concepts for one mechanism, with two identifiers, two loads
  and two write-backs per request.
- **One cookie.** A login would survive a browser restart whenever the session also held a long-lived
  value.
- **A long clock that starts only when the browser presents the cookie again.** It keeps cookieless
  bots out of the store, but a visitor who saw one page loses their consent choice.
- **Converting sessions written with one lifetime.** Novis is pre-alpha, and such a session expires
  within its two hours.
