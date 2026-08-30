---
summary: digests and HMACs over text or bytes, answered as raw `bytes` — the algorithm is an argument, never part of the member's name
keywords: hash, md5, sha1, sha256, crc32, hash_hmac, hash_equals, openssl_digest, digest, HMAC, checksum, constant-time comparison
---

`Core\Hash::of` computes any `Core\Digest` case over a `string` or `bytes` and answers the raw
digest as `bytes` — never hex and never an `int` — so printing one goes through
`Core\Encoding::toHex`. `hmac` takes only the strong cases: `Core\Digest::Md5`, `Sha1`, `Crc32`,
`Crc32c` and `Blake3` are compile errors there, while `of` accepts every case for checksum interop.
Compare two digests with `equals`, which runs in constant time. Input that arrives in pieces goes
through `stream`, which opens a `Core\Hash\Stream`.

```nvs
<?nvs
bytes $digest = Core\Hash::of("abc", Core\Digest::Sha256);
echo Core\Encoding::toHex($digest), "\n";
echo Core\Encoding::toHex(Core\Hash::of("abc", Core\Digest::Md5)), "\n";
echo Core\Encoding::toHex(Core\Hash::of("abc", Core\Digest::Crc32)), "\n";

bytes $key = "Jefe" as bytes;
bytes $mac = Core\Hash::hmac("what do ya want for nothing?", $key, Core\Digest::Sha256);
echo Core\Encoding::toHex($mac), "\n";

bytes $again = Core\Hash::of("abc", Core\Digest::Sha256);
echo Core\Hash::equals($digest, $again) ? "same" : "differ", "\n";
echo Core\Hash::equals($digest, Core\Hash::of("abd", Core\Digest::Sha256)) ? "same" : "differ", "\n";
```
```output
ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
900150983cd24fb0d6963f7d28e17f72
352441c2
5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843
same
differ
```
