Returns the options a browser needs to sign in with a passkey, as a JSON string.

Send the JSON to the page. There, JavaScript passes it to
`PublicKeyCredential.parseRequestOptionsFromJSON()`, and the result to
`navigator.credentials.get()`. The browser asks the user to choose a passkey and gives back its
answer. Read the passkey id in that answer with `Core\Passkey::credentialId`, and check the answer
with `Core\Passkey::verifySignIn`.

The options contain a new random challenge, which is saved in the session. So call
`Core\Session::start()` first. A challenge can be checked once, and only for five minutes. A new
call replaces the challenge of the last call. The options also ask the device to check that the
user is really there, with a fingerprint, a face or a PIN.

**Good to know:** when you know the account, pass its passkeys as `$allowed`. The browser then
offers only those. When `$allowed` is empty, the browser shows every passkey it has for your site.

**The examples below** show the options for any passkey, the options for the passkeys of one
account, and both halves of a sign-in page.
