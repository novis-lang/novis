A user's own secret — the access and refresh token a web application holds on their behalf — lives in their
session, through `Core\Session::setShortLivedSecret(string $key, secret string $value, array<secret bytes> $keys)`,
`Core\Session::setLongLivedSecret` with the same parameters, and
`Core\Session::getSecret(string $key, array<secret bytes> $keys)`, and only **sealed**. Static like
every member of that class (`rule:core-api/session-roster`): the session is the request's, not an object
a program holds. The construction is the
cache's (`rule:concurrency/a-secret-is-cached-only-sealed`) under the session door's own domain byte, so a
value sealed for a cache never opens as a session value and the reverse.

It is the session and not a cache tier because a cache may evict at any time and a user would be logged out
by a footprint decision. There is no `ttl`: a sealed value has the lifetime of the member that wrote it
(`rule:http-server/a-session-value-is-short-lived-or-long-lived`), and the sealed plaintext carries no
expiry of its own. The last write of a key sets its lifetime, a sealed one's too.

**A long-lived secret opens with the identifier cookie alone.** It survives the browser closing, so
whoever holds the identifier cookie on that device after a restart reads it, with no short cookie and
no login. A secret that belongs to a login, such as an access token, is written with
`setShortLivedSecret`, which ends with the browser session and with `clearShortLived`.
`setLongLivedSecret` is for a secret the device keeps whoever uses it, and its card says so.

**The additional data is the domain byte ‖ the app ‖ the key, and not the session id**, because
`regenerate` and each clear issue a new id over the same record and a value bound to the old id would
stop opening at exactly the moment a login hardens. Moving a ciphertext between two sessions takes
write access to the store, which already means owning every session in it.

`get` answers `null` for a sealed value and `setShortLived` and `setLongLived` refuse a `secret`, so the
sealed members are the only door here too; a sealed value that does not open under the ring is absent
rather than an error. The record crosses the store as the byte carrier it already is
(`rule:http-server/a-session-store-answers-five-operations`), so no `secret` reaches the store and
`rule:security/secret-crosses-no-boundary` is unchanged.
