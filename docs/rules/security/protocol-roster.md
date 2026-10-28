A closed roster of application-layer security protocols lives in `Core`: signed and encrypted cookies
(authenticated encryption only, with key rotation), CSRF tokens, TOTP, JWT — signed and verified under a
shared key, and verified when another party issued it (`rule:security/jws-issued-subset`) — JWE
(`rule:security/jwe-compact-subset`), detached signatures over a canonical payload, and `Core\Passkey`,
the server half of a WebAuthn registration and sign-in
(`rule:core-classes/passkey-verifies-a-webauthn-ceremony`). Nothing joins it without meeting the
admission test (`rule:security/protocol-admission-test`).

One more entry is admitted and not built: `Core\WebPush`, one encrypted push message
(`rule:core-classes/web-push-sends-an-encrypted-message`).

Each entry is a protocol whose usual failure is a silent bug in the library, not in the application:
a JWT verifier that accepts an unsigned token returns a valid-looking claim set, and nothing goes wrong
until it does. Left to userland, such a protocol grows several implementations, and some of them carry
that flaw. Novis removes the class of bug rather than documenting it. The roster is a surface of its
own, and `Core\Crypto` stays primitives-only.

They live in `Core` rather than in an extension for two reasons, and the second is structural. Token
verification sits on the hot path of every authenticated request, so a boundary crossing per request
is a real cost paid universally. And a `secret` value may not cross an extension boundary
(`rule:security/secret-does-not-cross-an-extension`), so an extension-based signer would need a
reveal at every call site — turning a deliberately conspicuous escape hatch into boilerplate, which
destroys its value as a signal. A userland implementation is still possible and cannot be prevented;
the claim is not exclusivity but that the obvious, documented option is the correct one.

**Every entry in the first paragraph is registered and carries conformance cases**, `Jwt::verifyIssued` included —
so a token another party issued is verified against a `Jwt\KeySet` here rather than in userland.
What the roster still owes is not a member but the flow around one: fetching, caching and rotating
a key set are a package's job (`rule:security/protocol-admission-test`), and every member here is
stateless over the key it is handed.

**A signed cookie is bound to its name and its purpose.** `Core\SignedCookie::seal` and `::open`
take the cookie's name, and it goes into the associated data behind an octet that only cookies use.
A value sealed for one cookie does not open as another, and a CSRF token, a `Core\Crypto::seal`
message or a sealed cache or session secret does not open as a cookie, even under the same key. An
application that uses one key everywhere, which nothing stops, gets no confusion between them.

**Declined:**
- **Expiry sealed into a signed cookie.** The cookie's own `Max-Age` and the program's data cover a
  lifetime, and a sealed one would be one more parameter on both members. A cookie stays valid while
  its key is in the ring, and retiring the key is the revocation.
- **A key derived per purpose instead of the associated data.** It binds the purpose and not the
  cookie's name, and it costs a derivation per call.
- **Nothing beyond primitives.** It is the cleanest boundary, and the one every other language drew;
  the result there is a population of libraries with the same repeated flaw.
- **First-party extensions.** A signing key is `secret`, and a `secret` cannot cross an extension
  boundary, so every call would need a reveal (above).
- **Session-adjacent entries only**, cookies and CSRF with JWT and TOTP left out. That draws the line
  at what Novis already implements, not at where the risk is, and JWT has the worst failure history
  of the roster.
- **An open surface that grows as needs appear.** Without the admission test's boundary it becomes a
  framework, and every entry is a permanent compatibility obligation
  (`rule:security/protocol-admission-test`).
