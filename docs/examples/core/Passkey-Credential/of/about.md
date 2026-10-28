Creates a `Core\Passkey\Credential` again from the four values that you saved with the account.

When a user registers a passkey, `Core\Passkey::verifyRegistration` returns a `Core\Passkey\Credential`.
Save its `id()`, `publicKey()`, `algorithm()` and `signCount()` with the account. When the user
signs in, pass the four saved values to `of`. Then pass the passkey to `Core\Passkey::verifySignIn`.
After each sign-in, save the new `signCount()`.

`of` checks the values again. The public key must be a COSE key of the algorithm that you pass. An
ES256 key must be a point on the P-256 curve, and an Ed25519 key must be 32 bytes. The id must be
from 1 to 1023 bytes. The sign counter must be from 0 to 4294967295. If a check fails, `of` throws a
`LogicError`. A saved row that was changed, or saved with a mistake, does not become a passkey
that a user can sign in with.

The passkey contains no secret. A public key can only check signatures, so you can save it like any
other value.

**The examples below** create a passkey from its four values, save one as text and read it back,
and show the errors for values that do not fit together.
