`Core\Passkey` creates the options for a WebAuthn registration or sign-in and verifies what the browser
returns, for ES256 and Ed25519 credentials, without verifying attestation, and the application stores
the credential it returns.

| Member | Does |
|---|---|
| `registerOptions(string $rpId, string $rpName, bytes $userId, string $userName, array<Passkey\Credential> $existing = []): string` | the JSON for `navigator.credentials.create()`, with a fresh challenge saved in the session |
| `verifyRegistration(tainted string $response, string $rpId, string $origin): Passkey\Credential` | checks the browser's answer and returns the new credential |
| `signInOptions(string $rpId, array<Passkey\Credential> $allowed = []): string` | the JSON for `navigator.credentials.get()`, with a fresh challenge saved in the session |
| `credentialId(tainted string $response): tainted bytes` | the credential id the browser claims, to look up the stored credential |
| `verifySignIn(tainted string $response, Passkey\Credential $stored, string $rpId, string $origin): Passkey\Credential` | checks the signature against the stored credential and returns it with its sign counter updated |

`Passkey\Credential` is a value with `id` (`bytes`), `publicKey` (`bytes`, the COSE key), `algorithm`
(`Passkey\Credential::Algorithm::ES256` or `Ed25519`) and `signCount` (`int`), each read by the
method of that name, and `Passkey\Credential::of($id, $publicKey, $algorithm, $signCount)` builds a
stored one again, checking the key against the algorithm. `Core\Passkey` reads and writes no
storage of its own: the application saves the credential wherever it keeps users.

**What verification checks:** the client data's type, challenge and origin; the authenticator data's
relying-party id hash, the user-present flag and the user-verified flag, which the options always ask
for; and, for a sign-in, the signature over both, verified by the code `Core\Crypto` already uses. A
credential whose algorithm is neither ES256 nor Ed25519 is refused at registration. A sign counter that
does not increase, when either value is not zero, throws.

**The challenge is single-use.** It comes from the system random source and is saved in the started
session (`rule:core-classes/session-is-started-explicitly`). Verification takes it out of the session
before it checks anything, so a second verification of one challenge, a replayed response, throws.
The origin and relying-party id are the values the application passes, never anything the browser sent.

**What the browser sent is `tainted`**, and the credential a successful verification returns is not.
The CBOR and COSE parsing comes from an existing crate
(`rule:packaging/somebody-elses-specification-is-a-crate`).

`Core\Passkey` is admitted to the protocol roster (`rule:security/protocol-roster`). Its usual failure
is a silent library bug — an unchecked origin, a skipped flag, a challenge accepted twice returns a
signed-in user — and every web application that signs people in needs it. The registry signs owners
in with it (`rule:packaging/a-registry-account-is-a-passkey`).

**Memory:** one challenge per ceremony, in the session, until it is verified.

**Declined:**
- **Attestation verification.** It needs a vendor certificate list that changes over time, and a
  registry or web application has no use for knowing the authenticator's maker.
- **Every COSE algorithm.** ES256 and Ed25519 cover current authenticators, and each further algorithm
  is more parsing to get wrong.
- **Credential storage inside `Core`.** Where users are kept is the application's choice, and a `Core`
  table would be a second user store beside it.
- **A challenge the application creates and passes back in.** Nothing could then stop one challenge
  from verifying twice.

**Not on disk.** There is no `Core\Passkey`.
