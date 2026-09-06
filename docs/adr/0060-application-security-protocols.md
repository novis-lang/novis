# ADR 0060 — A closed roster of application-layer security protocols lives in `Core`

- **Status:** Accepted
- **Date:** 2026-08-23
- **Scope:** which protocols built *on* cryptographic primitives ship in `Core`, the test that admits one,
  and the design constraints every entry must satisfy. Not in scope: `Core\Crypto`'s primitive roster, and
  not the method signatures, which M8 designs.
- **Amends:** [0051](0051-standard-library-tiers.md) — § 3's Core roster gains these entries and, in § 2
  below, the test that closes the list.
- **Amended by:** 0146

> **In short:** `Core` includes five protocols built on cryptographic primitives — **signed and encrypted
> cookies, CSRF tokens, TOTP, JWT signing and verification, and a detached signature over a payload** —
> and the list is **closed**: adding to it
> takes a new ADR. Each is admitted by a three-part test, and each must be **correct by construction**
> rather than correct by careful use: a JWT's algorithm comes from the key type and never from the token
> header, so `alg: none` and RS256→HS256 confusion are unrepresentable rather than defended against. The
> boundary that keeps the list closed is sharp: **stateless token operations over a key are in; multi-step
> flows with network I/O and stored state — OAuth, OpenID Connect, SAML, WebAuthn — are out**, and belong
> in userland or an extension.

## Context

- [ADR 0051](0051-standard-library-tiers.md) argues that a `Core` entry earns its place by needing runtime
  privilege, being an injection sink, or being too fine-grained for a boundary. None of those describes
  JWT, and yet leaving it out has a predictable outcome: the ecosystem grows several implementations, and
  some of them will accept `alg: none`. That has happened in essentially every language that left it to
  userland, PHP included.
- The distinguishing property is that these failures are **library** bugs rather than application bugs, and
  they are **silent**: a JWT verifier that accepts an unsigned token returns a valid-looking claim set, and
  nothing observable goes wrong until it does. That is a different risk profile from an ordinary library.
- The counter-pressure is real and this ADR takes it seriously. "Protocols people commonly get wrong" has
  no natural edge — it would grow to OAuth clients, SAML, WebAuthn, and eventually to a framework. A rule
  that admits these five and stops needs to state *why* it stops, or it will not.

## Decision

### 1. The roster

- **Signed and encrypted cookies** — AEAD only, with key rotation (verify against several keys, sign with
  the newest). No unauthenticated encryption mode exists in the API.
- **CSRF tokens** — generated against the session, compared in constant time, with the comparison being the
  only exposed operation so a caller cannot write `==`.
- **TOTP** — generation and verification with a bounded replay window and constant-time comparison.
- **JWT** — signing and verification, subject to § 4.
- **Detached signatures** — `Core\Signature`, over a canonical payload rather than over any assembled
  text, with the same key ring. [ADR 0146](0146-a-signature-is-over-a-payload-and-a-url-is-a-payload-core-uri.md)
  owns it and the two doors onto it that `Core\Uri` and `Core\Router` carry; it is admitted here because
  the failure history is a canonicalization bug in the library, silent, and near-universal for signed
  links.

### 2. The test, and why it closes

All three must hold:

1. **The failure mode is a library bug, not an application bug** — the mistake lives in the implementation
   of the protocol, not in how the application uses it.
2. **The failure is silent** — a wrong implementation returns a plausible result rather than an error.
3. **The need is near-universal** for the kind of program Novis exists to run.

And one structural boundary decides the edge: **the operation is stateless over a key**. A protocol
requiring network round trips, a redirect dance, or stored per-flow state is a *flow*, not a token
operation. OAuth 2, OpenID Connect, SAML and WebAuthn are all flows; they need HTTP clients, discovery
documents, nonce storage and expiry policy, and their designs vary per provider. They are out, permanently,
and are ordinary userland or extension code built on this roster plus `Core\Http\Client`.

### 3. Why `Core` and not a first-party extension

Two reasons, and the second is structural rather than a preference.

- Token verification sits on the hot path of **every authenticated request**, so a boundary crossing per
  request is a real if small cost paid universally.
- [ADR 0055](0055-extension-qualifier-declarations.md) § 3 refuses `secret` at every extension boundary.
  Signing keys are `secret` under [ADR 0033](0033-secret-qualifier-for-confidential-values.md), so an
  extension-based signer would require `Core\Secret::reveal()` at every call site — turning the
  deliberately conspicuous escape hatch into boilerplate, which destroys its value as a signal. That
  outcome is worse than the surface this ADR adds.

### 4. Correct by construction, not by careful use

Every entry is designed so the historical failure is **unrepresentable**, not merely defended against:

- **The algorithm comes from the key, never from the token.** A JWT's `alg` header is *checked against* the
  key's algorithm and rejected on mismatch; it is never consulted to select one. This removes `alg: none`
  and the RS256→HS256 confusion class in one stroke, because there is no code path in which an
  attacker-supplied string selects a verifier.
- **Expiry is mandatory for JWT.** A token without `exp`, or past it, fails verification. There is no
  flag to disable the check; a caller wanting a non-expiring credential is not using JWT for what JWT
  is. **This is JWT's rule and not the roster's** — its reasoning is about what a JWT is for, and
  `Core\SignedCookie` has carried no lifetime since it landed.
  [ADR 0146](0146-a-signature-is-over-a-payload-and-a-url-is-a-payload-core-uri.md) § 3 is where a
  lifetime is instead *written and never omitted*, `null` included.
- **Verification returns claims or throws.** It never returns a falsy value that a loose comparison could
  mistake for success — the same reasoning [ADR 0056](0056-regex-engine-policy.md) § 2 applies to a
  budget exhaustion.
- **Every comparison of a secret-derived value is constant-time**, and no API exposes the raw value for the
  caller to compare themselves.

### 5. A verified signature does not launder

Claims returned from JWT verification are **`tainted`**. A signature proves origin, not safety for any
sink: the payload may be authored by a third-party issuer, and even a self-issued token routinely carries
user-supplied data. Treating verification as laundering would be exactly the false confidence
[ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 3 refuses to allow anywhere else.

Cookie payloads the application itself sealed are the one case where the value round-trips through our own
AEAD unchanged, and they are returned **unqualified** — sealing is an authenticated operation over a value
that was already plain when it went in.

## Consequences

- **Five small, permanent API surfaces**, each closed to configuration in the places where configuration is
  how these protocols get broken.
- **The closed list will be argued with.** Someone will want OAuth in `Core`, and § 2's boundary is the
  answer: it is a flow. Recording the boundary now is what makes the answer a decision rather than a
  negotiation.
- **A JWT implementation in userland is still possible** and cannot be prevented. The claim here is not
  exclusivity but that the obvious, discoverable, documented option is the correct one.
- **`Core\Crypto` stays primitives-only.** This ADR does not open it to protocol-shaped additions; the
  roster is its own surface, and the distinction should survive M8's API design.
- **`nvs convert` (M11)** maps common PHP JWT libraries onto `Core`'s API, and emits a diagnostic where the
  source relied on header-selected algorithms or disabled expiry — behaviours with no representation here.

## Alternatives rejected

- **Nothing beyond primitives.** The cleanest boundary: `Core` owns cryptography, applications own
  protocols. Rejected: it is exactly the boundary every other language drew, and the observed outcome is a
  population of libraries with the same repeated flaw. Novis's whole argument is that classes of bug should
  be removed rather than documented.
- **First-party extensions.** Keeps `Core` smaller and matches [ADR 0051](0051-standard-library-tiers.md)'s
  treatment of internationalization. Rejected on § 3's second point: `secret` cannot cross the extension
  boundary, and making `Core\Secret::reveal()` routine would break the one mechanism that makes a
  credential's exposure visible.
- **Session-adjacent only** — cookies and CSRF, since `Core\Session` is core anyway; JWT and TOTP out.
  Rejected: it draws the line at what Novis already implements rather than at where the risk is, and JWT is
  the entry with the worst failure history of the five.
- **An open "security protocols" surface**, growing as needs appear. Rejected: without § 2's boundary it
  becomes a framework, and every entry is a permanent compatibility obligation.

## Verification

- **M8:** a fixture per failure class — a token with `alg: none` fails verification; a token signed with
  HMAC fails against an RSA public key with a diagnostic naming the mismatch rather than verifying; a token
  without `exp` fails; a token past `exp` fails; verification of a valid token returns claims that a
  `Core\Db` query-text position then **rejects** as tainted (§ 5).
- **M8:** a cookie sealed and opened round-trips unqualified; the same cookie with one payload byte altered
  fails to open; a cookie signed under a rotated-out key still verifies while new cookies use the newest
  key.
- **M8:** timing-sensitive comparisons are covered by a test asserting the API exposes no raw-value
  accessor, since constant-time behaviour itself is not reliably testable in CI.
