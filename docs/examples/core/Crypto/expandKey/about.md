Makes a key from material that is already random, such as a shared secret or a root key held by a
service.

You give it three things: the material, a salt, and a short text that names what the key is for. The
salt may be empty and is not a secret. The text is what keeps two keys apart: the same material with
the text `"messages"` and with the text `"file names"` gives two keys that have nothing to do with
each other. That is how one secret can protect several things at once, and it is better than using
the secret itself twice.

The result is 32 bytes, ready for `Core\Crypto::seal`. The same three arguments always give the same
key.

**Good to know:** the material has to be random already. A password is not, however long it is. Use
`Core\Crypto::deriveKey` for that, which stretches it first.
