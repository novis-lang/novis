Reads a JSON Web Key Set (JWKS), the list of public keys a login service publishes. It returns a
`Core\Jwt\KeySet` that you pass to `Core\Jwt::verifyIssued` to check the tokens of that service.

Each key in the list usually has a name in `kid`. A token names its key in the same way, so
`verifyIssued` uses exactly that one key. A token with no `kid` works only with a set that has
exactly one key.

`read` skips a key it does not use, for example a key for encryption. It throws a `RuntimeError`
when the whole document is wrong: it is not JSON, it has no `keys` list, two keys have the same
name, it contains a private key, or it has more than 16 keys. The message says what is wrong,
because the document comes from a service you chose.

**The examples below** read a key set with two keys, show the errors for a wrong document, and
load the key set of a login service from a file when a program starts.
