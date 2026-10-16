Encrypts a text and returns it as a JWE token, a short `string` you can put in a cookie, a URL or a
message. Only a program with the right key can read the text again, with `Core\Jwe::decrypt`.

The key is a `Core\Jwe\Key`. The method that made the key chooses the algorithm: `shared` for a
secret key that both sides already have, `password` for a password, and `recipient` for another
party's public key. The content is always encrypted with AES-GCM.

The token is different each time, even for the same text and the same key. So you cannot compare
two tokens to find out if their texts are equal.

**The examples below** encrypt a text with a shared key, encrypt a note with a password, and send
data to a partner who published a public key.
