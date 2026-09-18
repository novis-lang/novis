Names the construction a sealed message is written and read under.

`Core\Crypto::seal` and `Core\Crypto::open` each take a cipher, and there is no default: every call
says which of the two it is under. Both cases authenticate, so neither can produce a message somebody
could alter without the open being refused. What they differ in is who else can read the result.
`XChaCha20Poly1305` is the one to reach for when both ends are your own programs — its nonce is wide
enough that one is never drawn twice, and it is fast on a machine with no AES instructions.
`Aes256Gcm` is the one a browser can open, because it is what WebCrypto encrypts with.

**Good to know:** sealed bytes carry no label saying which case wrote them, so a program that stores a
message stores the case beside it. Opening under the other one is refused exactly as an altered
message is. The examples seal under both cases, show that refusal, and keep the case beside the
ciphertext in a small store.
