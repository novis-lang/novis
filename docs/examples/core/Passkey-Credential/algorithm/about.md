Returns the signature algorithm of the passkey, as a case of `Core\Passkey\Credential::Algorithm`.
The case is `ES256` or `Ed25519`.

The device chooses the algorithm when the user registers the passkey. The algorithm and the public
key belong together. `Core\Passkey\Credential::of` throws a `LogicError` when the public key is not
a key of the algorithm.

To save the case in a database, convert it to a number with `as int`. `ES256` is `0`, and `Ed25519`
is `1`. To read the number back, use `as Core\Passkey\Credential::Algorithm`. A number that is not
`0` or `1` throws a `RuntimeError`. With `as ?Core\Passkey\Credential::Algorithm`, the result is
`null`.

**The examples below** read the algorithm, save it as a number, and check a saved number.
