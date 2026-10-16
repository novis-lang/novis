A `secret` value meets a cache through exactly two members — `putSecret(string $key, secret string $value,
Duration $ttl, array<secret bytes> $keys)` and `getSecret(string $key, array<secret bytes> $keys, {fill?,
wait?})` — and what they store is **ciphertext**. The ring's newest key seals the value under
XChaCha20-Poly1305; the sealed plaintext is the value and its expiry, and the additional data is the
cache's domain byte ‖ the app ‖ the entry's name. The sealed bytes then enter the tier by its ordinary
path, as `bytes`.

So no `secret` ever crosses the cache boundary and
`rule:security/secret-crosses-no-boundary` is untouched: `put` still refuses a `secret` value, `get` still
answers `null` for a sealed entry, and the sealed pair is the only door. A `ttl` is required on
`putSecret`, because a secret never outlives a lifetime someone stated.

**A sealed entry that does not open is a miss**, never an error — under every key of the ring, past its
sealed expiry, or bound to another app or another name — so a rotated ring re-fetches rather than failing,
and a ciphertext moved between entries or apps answers nothing. A ring that is wrong in itself is still a
`LogicError`, because that is a bug in the program rather than a fact about the entry.

What sealing buys, exactly: a secret cannot be read by code that knows an entry's name but not the ring; it
can reach the shared tier without leaving the process in the clear; and a tampered, moved or replayed entry
is a miss. **It does not protect a secret from a compromised process** or a memory dump — the ring lives in
the same memory as the plaintext. What it costs is one seal per `putSecret` and one open per ring key tried
per `getSecret`, a ring ordered newest-first making the common case one open. A program that puts a
secret in a cache without a ring gets a compile error rather than a plaintext token in a network store,
which is the whole return on making the door narrow.

- **Memory:** a sealed entry costs its ciphertext: the value, a nonce, a tag, an eight-octet expiry and
  an eight-octet lifetime, held by whichever tier it was put in under that tier's cap.

**Declined:**
- **A tier that holds `secret` values in the clear**, since a process that holds the ring may as well
  hold the plaintext. That tier would have to differ from the one an ordinary value uses, or the boundary
  refusal would depend on which tier was asked for. Both are a second contract; the sealed door keeps one
  boundary rule and one payload type, and the tier does not know what it holds.
