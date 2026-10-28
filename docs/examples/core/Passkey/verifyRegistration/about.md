Checks the browser's answer to the options from `Core\Passkey::registerOptions`, and returns the
new passkey as a `Core\Passkey\Credential`.

In the browser, `navigator.credentials.create()` creates the passkey. Send
`JSON.stringify(credential.toJSON())` to the server, and pass that text as `$response`. Pass the
same domain as for `registerOptions`, and the address of your site, such as `https://example.com`.

Save the four values of the passkey with the account: `id()`, `publicKey()`, `algorithm()` and
`signCount()`. You need them when the user signs in.

**Good to know:** the check deletes the challenge from the session first. So a second check of the
same answer throws a `RuntimeError`. A check after a failed check also throws one.

**The examples below** show a new passkey, the same answer checked twice, and both halves of a
page that saves a passkey.
