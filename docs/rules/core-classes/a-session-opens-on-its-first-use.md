There is no ambient session array and no `start`. **The first call of any `Core\Session` member in a
request opens the session**: it reads the identifier the client presented and loads the record once,
and every later call in that request works on the record already in hand. A program that never calls
a member reaches no store and sends no cookie, so a request that does not use sessions pays nothing.

A program answering no request has no session. Its first call throws `LogicError` naming the member
(`rule:security/request-state-throws-in-an-isolate`), and the throw is catchable at the root like any
other.

**`openWithIdentifier($id)` is the one explicit open.** It opens the session under an identifier the
program read itself, such as a header, and no cookie is read or sent for it. It must be the request's
first `Core\Session` call: once the session is open, it throws `LogicError` naming the member that
opened it. A presented identifier the store did not issue is never used
(`rule:security/a-session-id-the-store-did-not-issue-is-rejected`).

**Opening is not storing.** A first call for a visitor with no session opens an empty one with no
identifier: no row is written and no cookie is sent. The first write that leaves something in it draws
the identifier and sends the cookie, and the end of the request writes the row. So the store grows with
sessions somebody wrote, never with requests served, and a flood of cookieless requests that only read
leaves nothing behind. `regenerate` on such a session does nothing, because no client holds an
identifier for it, and `clearShortLived` and `clearLongLived` have no row to change.

**A CSRF token counts as a write.** `Core\Csrf::token` binds its token to the identifier the response
sends, so on a session with none it draws one and sends the cookie, and it marks the session changed
even while the record is empty, so the end of the request writes the row and the next request finds
it. It spends one row per visitor who is shown a token, and `[session] short_lived` expires it.

**Declined:**
- **An explicit `start`.** A program that forgets it throws at its first read, and the explicit line
  bought nothing the first-use open does not keep: a request that calls no member still reaches no
  store, and a new visitor still gets no row and no cookie until a value is written.
- **An ambient session array.** It is a host-populated variable
  (`rule:statements/no-host-populated-variables`), and it would load the session for every request
  whether or not the program reads it.
