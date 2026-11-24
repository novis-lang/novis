Which hashing algorithm a `Core\Hash` call computes.

`Core\Hash::Digest` is an enum inside the class `Core\Hash`. `Core\Hash::of`, `Core\Hash::hmac` and
`Core\Hash::stream` take it as an argument.

You pass the algorithm as an argument, so one method works for every algorithm. The list has the
checksums, the two broken algorithms `Md5` and `Sha1`, the SHA-2 and SHA-3 families, and BLAKE3. Use
`Sha256` when nothing else decides the choice for you.

`Crc32`, `Md5` and `Sha1` are in the list because other systems still use them. An old database
column, an ETag, a package manifest or another program's signature can need one of them.

**Good to know:** where the choice matters for security, the types decide for you. `Core\Hash::hmac`
takes only the ten SHA-2 and SHA-3 cases, so a broken algorithm there does not compile.
