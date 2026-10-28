Returns the options a browser needs to create a new passkey for an account, as a JSON string.

Send the JSON to the page. There, JavaScript passes it to
`PublicKeyCredential.parseCreationOptionsFromJSON()`, and the result to
`navigator.credentials.create()`. The browser asks the user to create the passkey and gives back
its answer. Check that answer on the server with `Core\Passkey::verifyRegistration`.

The options contain a new random challenge, which is saved in the session. So call
`Core\Session::start()` first. A challenge can be checked once, and only for five minutes. The
options also ask the device to check that the user is really there, with a fingerprint, a face or
a PIN.

**Good to know:** pass the passkeys the account already has as `$existing`. A device that already
has one of them does not create a second one.

**The examples below** show the options for a new account, an account that already has a passkey,
and both halves of a page that adds a passkey.
