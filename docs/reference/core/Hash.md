---
summary: digests and HMACs of text, bytes, a file, an upload or the request body, returned as raw `bytes`. The algorithm is an argument
keywords: hash, md5, sha1, sha256, crc32, digest, HMAC, checksum, fingerprint, constant-time comparison, hash a file, file checksum
---

`Core\Hash::of` computes a digest with any `Core\Hash::Digest` case. `Core\Hash::hmac` computes a
signature with a secret key. Both take a `string`, `bytes`, an open file, an uploaded file,
`Core\Request::bodyStream()`, or a list or loop that gives `bytes` pieces. A file is read from its
first byte, one piece at a time. So a large file is never in memory as a whole.

The result is the raw digest as `bytes`. It is not hex and not an `int`. To print it, convert it with
`Core\Encoding::toHex`. `hmac` takes only the strong algorithms. `Core\Hash::Digest::Md5`, `Sha1`,
`Crc32`, `Crc32c` and `Blake3` do not compile there. `of` takes every case, so you can check
checksums from older systems. `equals` compares two digests in constant time. For data your program
makes one piece at a time, `stream` opens a `Core\Hash\Stream`.

```nvs
<?nvs
bytes $digest = Core\Hash::of('abc', Core\Hash::Digest::Sha256);
echo Core\Encoding::toHex($digest), "\n";
echo Core\Encoding::toHex(Core\Hash::of('abc', Core\Hash::Digest::Md5)), "\n";
echo Core\Encoding::toHex(Core\Hash::of('abc', Core\Hash::Digest::Crc32)), "\n";

bytes $key = 'Jefe' as bytes;
bytes $mac = Core\Hash::hmac('what do ya want for nothing?', $key, Core\Hash::Digest::Sha256);
echo Core\Encoding::toHex($mac), "\n";

bytes $again = Core\Hash::of('abc', Core\Hash::Digest::Sha256);
echo Core\Hash::equals($digest, $again) ? 'same' : 'differ', "\n";
echo Core\Hash::equals($digest, Core\Hash::of('abd', Core\Hash::Digest::Sha256)) ? 'same' : 'differ', "\n";
```
```output
ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
900150983cd24fb0d6963f7d28e17f72
352441c2
5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843
same
differ
```
