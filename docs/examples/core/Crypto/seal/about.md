Encrypts a message and protects it from being changed, in one step.

You give `Core\Crypto::seal` three things: the message, a 32 byte key, and the cipher to use. There
is no mode, no padding and no IV to choose. The member draws a new nonce for every call and puts it
in front of the result. Sealing the same message twice therefore gives two different results, and
both of them open to the same message.

The result is 40 bytes longer than the message under `XChaCha20Poly1305`, and 28 bytes longer under
`Aes256Gcm`. Use `XChaCha20Poly1305` when a Novis program opens the message again. Use `Aes256Gcm`
when a browser has to read it.

`Core\Crypto::open` is the other half. It throws an error when one byte of the sealed message has
been changed, so nobody can alter what you sealed.
