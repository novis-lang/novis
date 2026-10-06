Computes a digest of some data: a short, fixed-size fingerprint that changes completely when the
data changes by even one byte.

You choose the algorithm with a `Core\Digest` case, such as `Core\Digest::Sha256`. The same data
and the same algorithm always give the same digest, on every computer. The result is `bytes`, not
text. To show it or store it as text, convert it with `Core\Encoding::toHex`.

**In plain words:** a digest is like a fingerprint for data. Two files with the same fingerprint
are the same file. You cannot rebuild the file from the fingerprint.

**Good to know:** `Md5`, `Sha1` and the two `Crc32` cases are there so you can talk to older
systems. Do not use them to protect anything. To store passwords, use `Core\Password`.
