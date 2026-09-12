---
summary: authenticated encryption under a cipher named by a closed enum, with no mode, padding or nonce argument — a key is a `secret bytes`, and a message that has been altered is refused rather than decrypted
keywords: crypto, encrypt, decrypt, seal, open, generateKey, aead, cipher, chacha20, poly1305, xchacha20, aes, aes-256-gcm, webcrypto, openssl, sodium, nonce, key, tamper, forgery, authenticated
---

`Core\Crypto` names its cipher with a case of `Core\Crypto\Cipher` and never with a string.
`openssl_encrypt($data, "aes-256-cbc", $key, 0, $iv)` puts the primitive in a string and the nonce at
the call site, which is how a program ends up with `aes-256-ecb` in one file, an all-zero IV in another
and no authentication in either — "encrypted" and "authenticated" become two decisions a caller can get
half right. Here the enum is closed and every case authenticates, so a misspelling is a compile error
and there is no unauthenticated spelling to reach for; the mode, the padding and the nonce stay the
library's, exactly as `Core\Password`'s parameters do.

There is no default, either. `XChaCha20Poly1305` is the one to prefer when both ends are Novis, and
`Aes256Gcm` is what a browser's WebCrypto reads — a choice a call makes rather than one it inherits from
whichever line was copied last.

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
seals differently every time and the caller never keeps a counter. Under `XChaCha20Poly1305` a sealed
message is exactly 40 octets longer than its plaintext; under `Aes256Gcm` it is 28, the shorter nonce
being WebCrypto's own IV.

```nvs
<?nvs
secret bytes $key = Core\Crypto::generateKey();
bytes $message = "attack at dawn" as bytes;

// No mode, no padding, no IV — and 40 octets of overhead, flat.
bytes $sealed = Core\Crypto::seal($message, $key, Core\Crypto\Cipher::XChaCha20Poly1305);
if (Core\Bytes::length($sealed) == Core\Bytes::length($message) + 40) {
    echo "sealed, 40 octets over\n";
}

if (Core\Crypto::open($sealed, $key, Core\Crypto\Cipher::XChaCha20Poly1305) == $message) {
    echo "and it opens\n";
}

// The nonce is drawn per call, so the same message never seals the same way.
if (Core\Crypto::seal($message, $key, Core\Crypto\Cipher::XChaCha20Poly1305) != $sealed) {
    echo "a fresh nonce every time\n";
}

// One octet short is a forgery, and an authenticated mode says so.
bytes $tampered = Core\Bytes::slice($sealed, 0, (Core\Bytes::length($sealed) as int) - 1);
try {
    bytes $forged = Core\Crypto::open($tampered, $key, Core\Crypto\Cipher::XChaCha20Poly1305);
    echo "tamper accepted\n";
} catch (RuntimeError $refused) {
    echo "tamper refused\n";
}

// So is an intact message under a key that did not seal it.
secret bytes $other = Core\Crypto::generateKey();
try {
    bytes $wrong = Core\Crypto::open($sealed, $other, Core\Crypto\Cipher::XChaCha20Poly1305);
    echo "opened under the wrong key\n";
} catch (RuntimeError $notThisKey) {
    echo "the wrong key is refused\n";
}

// And the same message under the cipher it was not sealed with. Nothing
// plausible comes back: a wrong cipher is a forgery like any other.
try {
    bytes $elsewhere = Core\Crypto::open($sealed, $key, Core\Crypto\Cipher::Aes256Gcm);
    echo "opened under the wrong cipher\n";
} catch (RuntimeError $notThisCipher) {
    echo "the wrong cipher is refused\n";
}
```
```output
sealed, 40 octets over
and it opens
a fresh nonce every time
tamper refused
the wrong key is refused
the wrong cipher is refused
```
