Checks the browser's answer to the options from `Core\Passkey::signInOptions` against a saved
passkey. It returns the passkey as a `Core\Passkey\Credential` with its new sign counter.

In the browser, `navigator.credentials.get()` signs the challenge. Send
`JSON.stringify(credential.toJSON())` to the server, and pass that text as `$response`. Use
`Core\Passkey::credentialId` to find the saved passkey, and pass it as `$stored`. Pass the same
domain as for `signInOptions`, and the address of your site, such as `https://example.com`.

The method checks the signature with the saved public key. It also checks the challenge, the
domain and the address. If one check fails, it throws a `RuntimeError`.

**Save the result again.** The device counts its sign-ins. The new counter must be higher than the
saved one, or the method throws. A counter that does not rise can mean a copied passkey. Some devices
always send `0`. When both counters are `0`, the sign-in succeeds.

**Good to know:** the check deletes the challenge from the session first. So a second check of the
same answer throws a `RuntimeError`.

**The examples below** show one sign-in, the same answer checked twice, and both halves of a
sign-in page.
