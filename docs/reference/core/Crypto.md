---
summary: authenticated encryption with no cipher, mode, padding or nonce argument — a key is a `secret bytes`, and a message that has been altered is refused rather than decrypted
keywords: crypto, encrypt, decrypt, seal, open, generateKey, aead, chacha20, poly1305, xchacha20, openssl, sodium, nonce, key, tamper, forgery, authenticated
---

`Core\Crypto` has three members and no cipher name anywhere in them. `openssl_encrypt($data,
"aes-256-cbc", $key, 0, $iv)` puts the primitive in a string and the nonce at the call site, which is
how a program ends up with `aes-256-ecb` in one file, an all-zero IV in another and no authentication in
either — "encrypted" and "authenticated" become two decisions a caller can get half right. Here the
primitive belongs to the library, exactly as `Core\Password`'s parameters do: `::seal` takes a message
and a key, and there is no unauthenticated spelling to reach for.

**A key is a `secret bytes`, and that is part of its type.** `::generateKey` answers one, and the
compiler will not let a program put it in a plain `bytes` — so a key cannot be echoed, logged, dumped or
carried into a `Throwable` message by accident. No member here removes the mark: a `secret` key goes into
`::seal` and stays a secret, which is why encryption is not a hole in the `secret` qualifier.

Sealing a `secret` *message* is the deliberate act, and it looks like one:
`Core\Secret::revealBytes($token, "sealed under a key the store cannot read")`. That reads like friction
and is the mechanism — turning a secret into bytes that leave the process is the one event worth being
able to grep for.

**`::open` answers the plaintext or it throws.** There is no `false` and no `?bytes`. A message altered
by a single octet — the cheapest tamper there is — is refused, where an unauthenticated mode would hand
back whatever is left of the plaintext and let the program act on it. Every way of failing to be
authentic produces one message: altered, truncated, or under the wrong key are indistinguishable on
purpose, because telling them apart tells a forger which half of the attempt landed.

Each `::seal` draws its own nonce and prefixes it to the answer, so the same message under the same key
seals differently every time and the caller never keeps a counter. A sealed message is exactly 40 octets
longer than its plaintext.

```nvs
<?nvs
secret bytes $key = Core\Crypto::generateKey();
bytes $message = "attack at dawn" as bytes;

// No mode, no padding, no IV — and 40 octets of overhead, flat.
bytes $sealed = Core\Crypto::seal($message, $key);
if (Core\Bytes::length($sealed) == Core\Bytes::length($message) + 40) {
    echo "sealed, 40 octets over\n";
}

if (Core\Crypto::open($sealed, $key) == $message) {
    echo "and it opens\n";
}

// The nonce is drawn per call, so the same message never seals the same way.
if (Core\Crypto::seal($message, $key) != $sealed) {
    echo "a fresh nonce every time\n";
}

// One octet short is a forgery, and an authenticated mode says so.
bytes $tampered = Core\Bytes::slice($sealed, 0, (Core\Bytes::length($sealed) as int) - 1);
try {
    bytes $forged = Core\Crypto::open($tampered, $key);
    echo "tamper accepted\n";
} catch (RuntimeError $refused) {
    echo "tamper refused\n";
}

// So is an intact message under a key that did not seal it.
secret bytes $other = Core\Crypto::generateKey();
try {
    bytes $wrong = Core\Crypto::open($sealed, $other);
    echo "opened under the wrong key\n";
} catch (RuntimeError $notThisKey) {
    echo "the wrong key is refused\n";
}
```
```output
sealed, 40 octets over
and it opens
a fresh nonce every time
tamper refused
the wrong key is refused
```
