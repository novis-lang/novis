A closed roster of application-layer security protocols lives in `Core`: signed and encrypted cookies
(authenticated encryption only, with key rotation), CSRF tokens, TOTP, JWT — signed and verified under a
shared key, and verified when another party issued it (`rule:security/jws-issued-subset`) — JWE
(`rule:security/jwe-compact-subset`), and detached signatures over a canonical payload. Nothing joins it
without meeting the admission test (`rule:security/protocol-admission-test`).

They live in `Core` rather than in an extension for two reasons, and the second is structural. Token
verification sits on the hot path of every authenticated request, so a boundary crossing per request
is a real cost paid universally. And a `secret` value may not cross an extension boundary
(`rule:security/secret-does-not-cross-an-extension`), so an extension-based signer would need a
reveal at every call site — turning a deliberately conspicuous escape hatch into boilerplate, which
destroys its value as a signal. A userland implementation is still possible and cannot be prevented;
the claim is not exclusivity but that the obvious, documented option is the correct one.

**Signed cookies, CSRF, TOTP and JWT's shared-key half are on disk**, each with conformance cases.
`Core\Signature`, `Core\Jwe` and `Jwt::verifyIssued` are not: no member, no module, no case. The roster
is therefore not yet the closed set this rule describes.
