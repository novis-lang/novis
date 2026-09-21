Gives you the half of this key pair that you send to other people.

A key pair has a private half and a public half. The private half stays inside your program. It
signs messages with `Core\Crypto::sign`, and it agrees a shared secret with `Core\Crypto::agree`.
The public half is the one you publish. Anybody who has it can check your signatures, or agree that
same shared secret with you.

The public half is worked out from the private key each time you ask for it. It is not stored beside
the key. A pair can therefore never give you a public key that belongs to a different pair.

`Core\Crypto\PublicKey::write` turns the result into bytes you can send, in the format you name. The
other side reads those bytes back with `Core\Crypto\PublicKey::read`.
