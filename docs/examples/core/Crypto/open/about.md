Checks a sealed message and gives back what was sealed, or throws an error.

`Core\Crypto::open` is the other half of `Core\Crypto::seal`. You give it the sealed bytes, the same
32 byte key, and the same cipher. There is no IV to pass back in, because the nonce travels in front
of the sealed bytes.

The check comes first. If one byte of the sealed message has been changed, if it is too short, if
the key is not the one that sealed it, or if the cipher is not the one that sealed it, the member
throws a `RuntimeError`. It never gives back a half-decrypted message, and the error is the same
sentence in all four cases. Telling them apart would tell a forger which part of the guess was
right.

So a sealed message can travel anywhere. A program that opens one knows the bytes are exactly the
bytes somebody sealed under that key.
