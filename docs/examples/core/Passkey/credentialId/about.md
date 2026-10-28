Returns the passkey id in the browser's answer to `Core\Passkey::signInOptions`, as bytes.

Use the id to find the saved passkey and its account. Then pass the answer and that passkey to
`Core\Passkey::verifySignIn`. The id is what the browser sent, and nothing checks it here. So when
the answer comes from a request, the result is `tainted`. Compare it with the ids you saved, or use
it as a key to look one up.

The answer is the JSON of `credential.toJSON()` in the browser. The id is from 1 to 1023 bytes. An
answer that is not JSON, or that has no valid `id`, throws a `RuntimeError`.

**The examples below** read the id from an answer, find the account it belongs to, and show both
halves of a sign-in page.
