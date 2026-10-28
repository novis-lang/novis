Returns the public key of the passkey, as bytes. The bytes are in the COSE format, which is the
key format that WebAuthn uses.

The device sends this key when the user registers the passkey. At each sign-in,
`Core\Passkey::verifySignIn` checks the signature with it. An `Ed25519` key is usually 42 bytes, and
an `ES256` key is usually 77 bytes.

Save the bytes unchanged, with the other three values of the passkey. Pass them to
`Core\Passkey\Credential::of` to create the passkey again. `of` checks that the bytes are a key of
the saved algorithm. If they are not, it throws a `LogicError`.

**Good to know:** the key is public, so it does not need to be secret. But nobody else may change
it. Somebody who replaces the saved key with their own key can sign in to the account. Save it
where only your application can write.

**The examples below** read the key, compare the two kinds of key, and save the key as text.
