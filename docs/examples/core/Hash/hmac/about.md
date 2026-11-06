Computes a signature for some data with a secret key, so you can later check that nobody changed
the data.

This is called an HMAC. Only somebody who has the same key can compute the same signature. You
send or store the data together with its signature. When the data comes back, you compute the
signature again and compare the two with `Core\Hash::equals`. The result is `bytes`. Use
`Core\Encoding::toHex` or `Core\Encoding::toBase64Url` to put it in a URL or a header.

The data can be a `string`, `bytes`, an open file, an uploaded file, `Core\Request::bodyStream()`,
or a list or loop that gives `bytes` pieces. A file or a stream is read one piece at a time.

**In plain words:** a digest is a fingerprint that anybody can compute. An HMAC is a fingerprint
that only the owner of the key can compute.

**Good to know:** only the SHA-2 and SHA-3 algorithms are allowed. `Core\Hash::Digest::Md5` or
`Core\Hash::Digest::Sha1` here does not compile.
