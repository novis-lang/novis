The signature algorithm of a passkey: `ES256` or `Ed25519`.

`Core\Passkey\Credential::Algorithm` is declared in the class `Core\Passkey\Credential`. The method
`algorithm()` of a passkey returns it. `Core\Passkey\Credential::of` needs it as an argument, and
checks that the public key is a key of that algorithm.

`ES256` is ECDSA with the P-256 curve and SHA-256. Almost every device supports it. `Ed25519` is
the Ed25519 signature algorithm, and some devices support it. A passkey with any other algorithm
cannot be registered, so these two cases are all that a saved passkey can have.

**Good to know:** save the case with the other values of the passkey. A `match` over the two cases
turns it into text for a database, and another `match` turns the text back into the case.
