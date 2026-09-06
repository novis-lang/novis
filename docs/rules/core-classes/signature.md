`Core\Signature::sign` takes a map and an options bag carrying a key ring and an optional expiry, and
answers a token; `verify` takes a token and a key ring and answers the payload or throws.

**The input is a map, never text.** `sign` canonicalizes the payload itself — keys sorted, each value
encoded with its type — so there is no assembled string for the two sides to disagree about, which is
what every hand-rolled signing helper gets wrong. **The lifetime is inside the signed bytes**, not
beside them, so a holder cannot edit it and there is no second parameter to keep in step. The token
is URL-safe by construction — unpadded URL-safe base64 — so every octet is a legal cookie octet and a
legal query-string value and nothing downstream escapes it again. The key ring is newest-first:
signing uses the head, verifying tries the ring in order, rotating is prepending and retiring is
dropping the tail. One rotation vocabulary in the language, not two.

**A verified payload is `tainted`.** A token minted by one service and read by another is not the
signed-cookie case, where the plaintext was the application's own when it went in; "we authored this
payload" is not a property the checker can see, so it is not one the return type may assume.

**Not shipped.** There is no `Core\Signature` in `crates/nvs-stdlib/src/`; the nearest landed member
is `signed_cookie.rs`, whose key-ring shape this one adopts.
