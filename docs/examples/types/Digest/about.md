Which hashing algorithm a `Core\Hash` call computes.

The algorithm is an argument rather than part of the member's name, so one member stands where other
languages have a function per algorithm. The list holds every algorithm interop needs: the checksums,
the two broken ones, the SHA-2 and SHA-3 families, and BLAKE3. `Sha256` is the one to reach for when
nothing else decides it for you.

`Crc32`, `Md5` and `Sha1` are in the list on purpose. A legacy database column, an ETag, a package
manifest or somebody else's signature scheme names one of them, and a language that refuses to
compute them does not stop anyone using them — it only makes the program do it some worse way.

**Good to know:** where the choice is a security one, the type system decides for you. `Core\Hash::hmac`
takes only the ten SHA-2 and SHA-3 cases, so a broken algorithm there does not compile.
