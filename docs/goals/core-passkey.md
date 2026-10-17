---
looper: 1
title: "`Core\\Passkey` registers and verifies a passkey"
summary: "Core\\Passkey creates WebAuthn registration and authentication options and verifies both ceremonies for ES256 and Ed25519 credentials without verifying attestation, the application stores the credential it returns, the class is on the protocol roster and has its feature proofs, rule core-classes/passkey-verifies-a-webauthn-ceremony is shipped, and the checks below pass."
context: [crates/nvs-stdlib/src/passkey.rs, docs/rules/core-classes/passkey-verifies-a-webauthn-ceremony.md]
stages:
  - id: 1
    title: a registration is created and verified, and the credential is returned to the application
    status: open
    context: [crates/nvs-stdlib/src/totp.rs, crates/nvs-stdlib/src/crypto.rs, crates/nvs-stdlib/src/registry.rs, Cargo.toml]
    extra:
      context:
        rules: [core-classes/passkey-verifies-a-webauthn-ceremony, packaging/somebody-elses-specification-is-a-crate, security/protocol-roster]
  - id: 2
    title: an authentication is verified against a stored credential
    status: open
    context: [crates/nvs-stdlib/src/crypto.rs]
    extra:
      context:
        rules: [core-classes/passkey-verifies-a-webauthn-ceremony]
  - id: 3
    title: the inputs are tainted, the challenge is single-use, and the class is on the protocol roster
    status: open
    context: [crates/nvs-stdlib/src/taint.rs, docs/rules/security/protocol-roster.md, docs/rules/security/protocol-admission-test.md]
    extra:
      context:
        rules: [core-classes/passkey-verifies-a-webauthn-ceremony, security/protocol-roster, security/protocol-admission-test]
  - id: 4
    title: "`Core\\Passkey` has its feature proofs, and the rule is shipped"
    status: open
    context: [docs/examples/core/Totp/check/about.md, tests/hostile/core/Totp/check/01-guess-every-code.nvs]
checks:
  - id: passkey-proofs
    stage: 4
    name: Core\Passkey owes no feature proof
    run: [bun, nv, proofs, --gate, --group, "Core\\Passkey"]
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, core-passkey]
    reach: true
    floor: false
extra:
  milestone: M15
---

# `Core\Passkey` registers and verifies a passkey

## Why

The registry's accounts sign in with passkeys and nothing else (goal `registry-service`). A passkey
is a WebAuthn credential: the browser signs a challenge with a key that never leaves the user's
device. Getting the checks right is the hard part, and a wrong check returns a valid-looking login, so
the verification belongs in `Core` beside `Core\Totp` and `Core\Jwt` rather than in each application.
Every Novis web application gains it, not only the registry. The registry's Web Push notifications
are goal `core-web-push`'s, which keeps its AES-128-GCM inside `Core\WebPush`.

## Standing decisions

**The member names and shapes are the ones
`rule:core-classes/passkey-verifies-a-webauthn-ceremony` states.** The class covers both ceremonies:
options for the browser, then verification of what the browser returns.

**ES256 and Ed25519 only, and no attestation verification.** A credential with another algorithm is
refused at registration. The attestation statement is not checked, so no vendor certificate list is
shipped or updated.

**The application stores credentials.** `Core\Passkey` reads and writes no storage. Registration
returns a credential value (its id, its public key, its algorithm and its sign counter), and
authentication takes that value and returns it with the counter updated. The application saves it
wherever it keeps users.

**The challenge is created by `Core\Passkey`, comes from the system random source, and is accepted
once.** The application keeps it in the session between the two requests. The origin and the relying
party id are checked against values the application passes, never against anything the browser sent.

**The CBOR and COSE parsing comes from an existing crate**, chosen under
`rule:packaging/somebody-elses-specification-is-a-crate` and
`rule:packaging/a-c-dependency-answers-two-questions`. Signatures are verified by the code
`Core\Crypto` already uses (`crates/nvs-stdlib/src/crypto.rs:2620`).

**Trade-offs.** A web application gains passwordless sign-in with no package and no JavaScript library
on the server. Latency: one signature check per sign-in, on the request that signs in. Memory: the
challenge, per request, in the session. Simplicity: one class with a small surface; the browser half
stays the browser's `navigator.credentials` API, which the examples show.

## Stages

### 1. a registration is created and verified, and the credential is returned to the application

**Does:** the class creates registration options and verifies the browser's response, returning a
credential value.

- [ ] The CBOR crate chosen and recorded in its manifest row, with `cargo deny` green. `Cargo.toml:1`
- [ ] The options, and the checks of `clientDataJSON` (type, challenge, origin) and of the authenticator data (relying party id hash, user present, user verified when asked). `crates/nvs-stdlib/src/passkey.rs:1`
- [ ] The COSE key read for ES256 and Ed25519, any other algorithm refused, and the credential value returned (`rule:core-classes/passkey-verifies-a-webauthn-ceremony`). `crates/nvs-stdlib/src/passkey.rs:1`

### 2. an authentication is verified against a stored credential

**Does:** the browser's assertion is verified against the credential the application stored, and the
updated credential is returned.

- [ ] The assertion's client data, authenticator data and signature over both. `crates/nvs-stdlib/src/passkey.rs:1`, `crates/nvs-stdlib/src/crypto.rs:2620`
- [ ] A sign counter that does not increase, when either value is not zero, throws. `crates/nvs-stdlib/src/passkey.rs:1`

### 3. the inputs are tainted, the challenge is single-use, and the class is on the protocol roster

**Does:** what the browser sent is `tainted`, a challenge verifies once, and the class meets the
roster's admission test.

- [ ] The browser's response is `tainted`, and the verified credential id is not. `crates/nvs-stdlib/src/passkey.rs:1`, `crates/nvs-stdlib/src/taint.rs:1`
- [ ] A challenge used twice, a wrong origin and a wrong relying party id each throw. `crates/nvs-stdlib/src/passkey.rs:1`
- [ ] The class is on `rule:security/protocol-roster` and passes `rule:security/protocol-admission-test`. `data/rules/security/protocol-roster.json:1`

### 4. `Core\Passkey` has its feature proofs, and the rule is shipped

**Does:** each member has its about page, tests, examples, bench, attack and reference card.

- [ ] Rust tests over a recorded browser ceremony for each algorithm, and conformance cases for each refusal. `crates/nvs-stdlib/src/passkey.rs:1`
- [ ] `bun nv proofs --group 'Core\Passkey'` owes nothing; the examples show the browser half beside the server half, in the shape of `Core\Totp::check`'s. `docs/examples/core/Totp/check/03-a-login-form.nvs:1`
- [ ] The attacks: a replayed registration and a replayed assertion, in the shape of `Core\Totp::check`'s. `tests/hostile/core/Totp/check/01-guess-every-code.nvs:1`
- [ ] The rule is `shipped`, and `bun nv owners --closes core-passkey` passes. `data/rules/core-classes/passkey-verifies-a-webauthn-ceremony.json:1`

## Handoff

### State

The goal is new. No stage is started. `Core\Totp` (`crates/nvs-stdlib/src/totp.rs:1`) is the nearest
class in shape.

### Next

Stage 1, the registration ceremony:
- choose the CBOR crate (`rule:packaging/somebody-elses-specification-is-a-crate`). `Cargo.toml:1`
- options and verification (`rule:core-classes/passkey-verifies-a-webauthn-ceremony`). `crates/nvs-stdlib/src/passkey.rs:1`

### Backlog

- The registry's sign-in uses this goal's work in goal `registry-service` (`docs/goals/registry-service.md`).
- Web Push, with its AES-128-GCM kept inside the class, is goal `core-web-push`'s (`docs/goals/core-web-push.md`).
