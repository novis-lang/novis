---
summary: the interop primitives — authenticated encryption under a cipher named by a closed enum, two key derivations, key pairs over three curves, agreement and signatures, with no algorithm ever a string and none with a default
keywords: crypto, encrypt, decrypt, seal, open, generateKey, deriveKey, expandKey, generateKeyPair, agree, sign, verify, publicKey, keypair, aead, cipher, chacha20, poly1305, xchacha20, aes, aes-256-gcm, webcrypto, openssl, sodium, nonce, key, tamper, forgery, authenticated, pbkdf2, hkdf, derivation, salt, iterations, hash_pbkdf2, hash_hkdf, p256, x25519, ed25519, rsa, ecdh, ecdsa, eddsa, curve, spki, pkcs8, jwk, signature, openssl_sign, openssl_verify, openssl_pkey_new, sodium_crypto_sign, sodium_crypto_box
---

`Core\Crypto` holds the primitives a Novis program shares with something that is not Novis: a browser's
WebCrypto, an SDK, whoever issued its keys. It seals and opens under an authenticated cipher, stretches a
password and spreads uniform material into keys, draws key pairs, agrees a shared secret over a curve, and
signs and verifies a message. **No algorithm is ever a string, and none has a default** — a cipher, a
key's kind and a key's encoding are each a case of a closed enum that the call names.

`openssl_encrypt($data, "aes-256-cbc", $key, 0, $iv)` puts the primitive in a string and the nonce at
the call site, which is how a program ends up with `aes-256-ecb` in one file, an all-zero IV in another
and no authentication in either — "encrypted" and "authenticated" become two decisions a caller can get
half right. Here the enum is closed and every case authenticates, so a misspelling is a compile error
and there is no unauthenticated spelling to reach for; the mode, the padding and the nonce stay the
library's, exactly as `Core\Password`'s parameters do. `XChaCha20Poly1305` is the one to prefer when both
ends are Novis, and `Aes256Gcm` is what a browser reads.

**A key is a `secret bytes`, and that is part of its type.** `::generateKey` answers one, and the
compiler will not let a program put it in a plain `bytes` — so a key cannot be echoed, logged, dumped or
carried into a `Throwable` message by accident. No member here removes the mark: a `secret` key goes into
`::seal` and stays a secret, and the private half of a pair leaves its `Core\Crypto\KeyPair` only as the
`secret bytes` of a PKCS#8.

Sealing a `secret` *message* is the deliberate act, and it looks like one:
`Core\Secret::revealBytes($token, "sealed under a key the store cannot read")`. That reads like friction
and is the mechanism — turning a secret into bytes that leave the process is the one event worth being
able to grep for.

**`::open` answers the plaintext or it throws.** There is no `false` and no `?bytes`. A message altered
by a single octet — the cheapest tamper there is — is refused, where an unauthenticated mode would hand
back whatever is left of the plaintext and let the program act on it. Every way of failing to be
authentic produces one message: altered, truncated, under the wrong key, or under the other cipher are
indistinguishable on purpose, because telling them apart tells a forger which half of the attempt landed.

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
```
```output
sealed, 40 octets over
and it opens
a fresh nonce every time
tamper refused
the wrong key is refused
```

## Two derivations, and which one a value belongs to

A key comes from one of three places: it is drawn, it is stretched out of something a person chose, or
it is spread out of something already uniform. `::generateKey` draws, `::deriveKey` stretches with
PBKDF2-HMAC-SHA256, and `::expandKey` spreads with HKDF-SHA256. All three answer the same 32 octets, so
whichever one produced a key, `::seal` takes it.

**They are not interchangeable, and the difference is what the input is.** A password has little entropy
and is guessed offline, so stretching it is slow on purpose: the iteration count is a required argument,
refused below 100,000 and above 2,000,000, and the salt is at least 16 octets. Nothing about a
`Core\Crypto::expandKey` call is slow — it is two HMAC runs — because the secret going into it is already
uniform, which a password is not. No type tells the two apart, so this is the one choice here a program
makes for itself.

`$info` is what makes one root secret into as many keys as a program has uses for. Two calls over one
secret with different context strings answer unrelated keys, which is how a service stops reusing one
secret at two call sites without storing a second one.

```nvs
<?nvs
// A password is stretched. The count is the call's, because whoever wrote a
// derived key down is the one who chose what it was derived under.
bytes $salt = Core\Random::bytes(16);
secret bytes $fromPassword = Core\Crypto::deriveKey("correct horse battery staple", $salt, 100000);

// Material that is already uniform is spread instead, and `$info` keeps two
// uses of one secret apart.
secret bytes $root = Core\Crypto::generateKey();
secret bytes $forCookies = Core\Crypto::expandKey($root, $salt, "cookies");
secret bytes $forTokens = Core\Crypto::expandKey($root, $salt, "tokens");
if ($forCookies != $forTokens) {
    echo "one secret, two keys\n";
}

// Either answer is a key like any other, and is a `secret bytes` like any
// other: neither derivation is a way out of the qualifier.
bytes $message = "attack at dawn" as bytes;
bytes $sealed = Core\Crypto::seal($message, $fromPassword, Core\Crypto\Cipher::XChaCha20Poly1305);
if (Core\Crypto::open($sealed, $fromPassword, Core\Crypto\Cipher::XChaCha20Poly1305) == $message) {
    echo "a derived key seals like a drawn one\n";
}

// The count is bounded on both sides. Under the floor the answer is cheap to
// attack; over the ceiling one call is a denial of service against the process
// that made it, which is what stops a token naming its own.
try {
    secret bytes $cheap = Core\Crypto::deriveKey("correct horse battery staple", $salt, 1000);
    echo "1000 rounds accepted\n";
} catch (LogicError $outside) {
    echo "1000 rounds refused\n";
}
```
```output
one secret, two keys
a derived key seals like a drawn one
1000 rounds refused
```

## A pair, and which half each member wants

A `Core\Crypto\KeyPair` is both halves of one key, and `Core\Crypto\PublicKey` is the half that is sent.
`::generateKeyPair` draws a pair of the kind it is given — `P256`, `X25519` or `Ed25519` — and refuses
both RSA kinds, which are read from whatever PKCS#8 the issuer wrote rather than drawn here. A pair
crosses a restart through `$pair->write()`, its PKCS#8, and a peer's key crosses the wire through
`$publicKey->write($format)` and `Core\Crypto\PublicKey::read($encoded, $kind, $format)` over `Raw`,
`Spki` and `Jwk` — WebCrypto's own three export encodings. **A public key is validated at `read`**: a
point that is not on the curve, a coordinate of the wrong width or an RSA modulus outside 2048 to 8192
bits never becomes a `Core\Crypto\PublicKey` at all, so no later member can be handed a key nobody
looked at.

<!-- src: rule:security/algorithm-comes-from-the-key -->
**The key's kind is the algorithm, at every member.** A `P256` or `X25519` pair agrees and signs nothing
or agrees nothing respectively; `::sign` writes ECDSA under `P256`, Ed25519 under `Ed25519` and
RSASSA-PKCS1-v1_5 or RSASSA-PSS under the RSA kind the key was read as. Nothing takes an algorithm
argument that could disagree with the key it was given, and a pair asked for the operation its kind does
not do is a `LogicError` rather than a weaker answer. `::agree` answers the shared secret as material for
`::expandKey`, never as a key: a raw ECDH output is biased and is spread before it seals anything.

```nvs
<?nvs
// Each end draws a pair and sends the half that is public, in the encoding the
// other end reads. `Raw` is the 32 octets a browser exports for a curve.
Core\Crypto\KeyPair $mine = Core\Crypto::generateKeyPair(Core\Crypto\KeyKind::X25519);
Core\Crypto\KeyPair $theirs = Core\Crypto::generateKeyPair(Core\Crypto\KeyKind::X25519);
bytes $sent = $mine->publicKey()->write(Core\Crypto\KeyFormat::Raw);

// What arrives is read back before anything uses it, under the kind and the
// encoding the protocol says it is in.
Core\Crypto\PublicKey $peer = Core\Crypto\PublicKey::read(
    $theirs->publicKey()->write(Core\Crypto\KeyFormat::Raw),
    Core\Crypto\KeyKind::X25519,
    Core\Crypto\KeyFormat::Raw
);
Core\Crypto\PublicKey $me = Core\Crypto\PublicKey::read(
    $sent,
    Core\Crypto\KeyKind::X25519,
    Core\Crypto\KeyFormat::Raw
);

// Both ends agree the same secret from opposite halves and spread it into a
// key, which is a key like any other: it seals here and opens there.
bytes $salt = "a salt both ends know" as bytes;
secret bytes $ours = Core\Crypto::expandKey(Core\Crypto::agree($mine, $peer), $salt, "session");
secret bytes $yours = Core\Crypto::expandKey(Core\Crypto::agree($theirs, $me), $salt, "session");
bytes $sealed = Core\Crypto::seal("ping" as bytes, $ours, Core\Crypto\Cipher::Aes256Gcm);
if (Core\Crypto::open($sealed, $yours, Core\Crypto\Cipher::Aes256Gcm) == ("ping" as bytes)) {
    echo "one secret, agreed from both ends\n";
}

// A signing pair is a different kind, and `::verify` answers nothing at all —
// there is no boolean here for a loose comparison to get wrong.
Core\Crypto\KeyPair $signing = Core\Crypto::generateKeyPair(Core\Crypto\KeyKind::Ed25519);
bytes $announcement = "the service says this" as bytes;
bytes $signature = Core\Crypto::sign($announcement, $signing);
Core\Crypto::verify($announcement, $signature, $signing->publicKey());
echo "signed and verified, ", Core\Bytes::length($signature), " octets\n";

// Another message under the same signature is a forgery, with the one sentence
// every refusal here gets.
try {
    Core\Crypto::verify("the service says something else" as bytes, $signature, $signing->publicKey());
    echo "another message took the signature\n";
} catch (RuntimeError $notThisMessage) {
    echo "another message is refused\n";
}

// And the X25519 pair signs nothing: that is a bug in the program, not a
// verdict on anyone, so it is a `LogicError`.
try {
    bytes $never = Core\Crypto::sign($announcement, $mine);
    echo "a curve for agreement signed\n";
} catch (LogicError $signsNothing) {
    echo "an agreement key signs nothing\n";
}
```
```output
one secret, agreed from both ends
signed and verified, 64 octets
another message is refused
an agreement key signs nothing
```
