A user's own secret — the access and refresh token a web application holds on their behalf — lives in their
session, through `Core\Session::setSecret(string $key, secret string $value, array<secret bytes> $keys)`
and `Core\Session::getSecret(string $key, array<secret bytes> $keys)`, and only **sealed**. Static like
every member of that class (`rule:core-api/session-roster`): the session is the request's, not an object
a program holds. The construction is the
cache's (`rule:concurrency/a-secret-is-cached-only-sealed`) under the session door's own domain byte, so a
value sealed for a cache never opens as a session value and the reverse.

It is the session and not a cache tier because a cache may evict at any time and a user would be logged out
by a footprint decision. There is no `ttl`: a session value lives as long as its session
(`rule:http-server/session-expiry-belongs-to-the-store`), and the sealed plaintext carries no expiry of its
own.

**The additional data is the domain byte ‖ the app ‖ the key, and not the session id**, because
`regenerate` issues a new id over the same record and a value bound to the old id would stop opening at
exactly the moment a login hardens. Moving a ciphertext between two sessions takes write access to the
store, which already means owning every session in it.

`get` answers `null` for a sealed value and `set` still refuses a `secret`, so the sealed pair is the only
door here too; a sealed value that does not open under the ring is absent rather than an error. The record
crosses the store as the byte carrier it already is
(`rule:http-server/a-session-store-answers-four-operations`), so no `secret` reaches the store and
`rule:security/secret-crosses-no-boundary` is unchanged.
