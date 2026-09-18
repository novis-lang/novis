Names which asymmetric key a call is talking about — the curve, or the RSA signing scheme.

Every member that makes, reads or uses a key pair takes one of these five cases, and there is no
default. The kinds do not stand in for one another: `X25519` agrees a shared secret with a peer and
signs nothing, `Ed25519` signs and agrees nothing, and `P256` does both, which is why it is the curve
an interop key is usually on. The two RSA cases are one key type under two schemes — `RsaPkcs1` is
JWS's `RS256` and `RsaPss` is `PS256` — and an RSA key is read from whoever issued it rather than
drawn here.

**In plain words:** a key is not a general-purpose thing you point at any operation. What it is for
is part of what it is, and the kind is how a program says so. Asking a key to do the other job is a
mistake in the program, reported as one. The examples put the five cases side by side, agree a secret
across two ends of one curve, and verify a partner's signature against the key on file.
