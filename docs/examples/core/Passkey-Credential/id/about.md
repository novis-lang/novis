Returns the id of the passkey, as bytes. The id is from 1 to 1023 bytes.

The device creates the id when the user registers the passkey. When the user signs in, the browser
sends the same id. `Core\Passkey::credentialId` reads it from the browser's answer. Compare that id
with the ids you saved to find the passkey and its account.

Save the id with the other three values of the passkey: `publicKey()`, `algorithm()` and
`signCount()`. A database column for bytes stores it unchanged. To save it as text, use
`Core\Encoding::toBase64Url`. The browser writes the id in the same way, so the two texts are equal.

**The examples below** read the id, find the passkey of a sign-in, and save the id as text.
