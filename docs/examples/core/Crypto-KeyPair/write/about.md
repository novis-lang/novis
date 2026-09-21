Gives you this pair's private key as bytes, so your program can store it and read it back later.

The result is `secret bytes`. Novis keeps a secret value out of your output, your logs and your
error messages. To get plain bytes from it, call `Core\Secret::revealBytes` and give a reason. That
reason stays in your code for the next person who reads it.

The bytes are PKCS#8, in the binary DER form. `Core\Crypto\KeyPair::read` takes them back, and so
does any other program that reads PKCS#8. A pair that was read from a PEM block writes the binary
form here, because that form is what the PEM block carried.

**Good to know:** a private key that leaves your program is a key somebody else can use. Store it
where only your server can read it, and never send it to a client.
