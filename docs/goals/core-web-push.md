---
looper: 1
title: "`Core\\WebPush` sends an encrypted, signed Web Push message"
summary: "Core\\WebPush encrypts a payload for a browser's push subscription as RFC 8291's aes128gcm encoding with the nonce derived inside and never a parameter, signs a VAPID token with the application's P-256 key as RFC 8292 says, and sends the message through Core's HTTP client under the ordinary net.connect grant, AES-128-GCM stays inside the class and is not a case of Core's cipher roster, the class is on the protocol roster, it has its feature proofs, rule core-classes/web-push-sends-an-encrypted-message is shipped, and the checks below pass."
context: [crates/nvs-stdlib/src/webpush.rs, docs/rules/core-classes/web-push-sends-an-encrypted-message.md]
stages:
  - id: 1
    title: a payload is encrypted as RFC 8291's `aes128gcm` encoding, with the nonce derived inside
    status: open
    context: [crates/nvs-stdlib/src/crypto.rs, Cargo.toml]
    extra:
      context:
        rules: [core-classes/web-push-sends-an-encrypted-message, core-classes/crypto-interop-tier]
  - id: 2
    title: a VAPID token is signed with the application's key, and the message is sent under `net.connect`
    status: open
    context: [crates/nvs-stdlib/src/crypto.rs, crates/nvs-stdlib/src/jwt.rs, crates/nvs-stdlib/src/http.rs]
    extra:
      context:
        rules: [core-classes/web-push-sends-an-encrypted-message, security/capability-check-at-the-door]
  - id: 3
    title: the subscription is tainted, the key is secret, and the class is on the protocol roster
    status: open
    context: [crates/nvs-stdlib/src/taint.rs, docs/rules/security/protocol-roster.md, docs/rules/security/protocol-admission-test.md]
    extra:
      context:
        rules: [core-classes/web-push-sends-an-encrypted-message, security/protocol-roster, security/protocol-admission-test]
  - id: 4
    title: "`Core\\WebPush` has its feature proofs, and the rule is shipped"
    status: open
    context: [docs/examples/core/Totp/check/about.md, tests/hostile/core/Totp/check/01-guess-every-code.nvs]
checks:
  - id: web-push-proofs
    stage: 4
    name: Core\WebPush owes no feature proof
    run: [bun, nv, proofs, --gate, --group, "Core\\WebPush"]
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, core-web-push]
    reach: true
    floor: false
---

# `Core\WebPush` sends an encrypted, signed Web Push message

## Why

The registry notifies owners by Web Push (goal `registry-service`). A push message is encrypted for
one browser subscription (RFC 8291) and carries a signed VAPID token that names the sender (RFC 8292).
Each step has a silent failure: a reused nonce, a wrong key derivation, or a token the push service
accepts for the wrong sender. So the whole message is built in `Core` beside `Core\Passkey`, and every
Novis web application can notify its users without a package. P-256 agreement, ECDSA signing and HKDF
are already in `Core\Crypto` (`crates/nvs-stdlib/src/crypto.rs:1808`, `crates/nvs-stdlib/src/crypto.rs:2977`,
`crates/nvs-stdlib/src/crypto.rs:1748`).

## Standing decisions

**The member names and shapes are the ones
`rule:core-classes/web-push-sends-an-encrypted-message` states.** The class takes a subscription (the
endpoint URL and the browser's `p256dh` and `auth` values), a payload and the application's VAPID key,
and sends one message.

**The nonce is derived inside and is never a parameter.** Each message draws a fresh salt and a fresh
P-256 key pair from the system random source, and the content key and the nonce come from HKDF as
RFC 8291 says. No `Core` cipher takes a caller-chosen nonce, and this class adds none.

**AES-128-GCM is internal to `Core\WebPush`.** It comes from the `aes-gcm` crate that already provides
AES-256-GCM (`crates/nvs-stdlib/src/crypto.rs:1601`). It is not a new case of `Core\Crypto\Cipher`,
so `rule:core-classes/crypto-interop-tier`'s roster does not change.

**The message is sent through `Core`'s HTTP client under the ordinary `net.connect` grant.** The
endpoint comes from a browser, so it is `tainted`, and the client's outbound address table refuses an
internal host as it does for any request (`crates/nvs-stdlib/src/http.rs:461`). The push service's
answer is returned, so the application can drop a subscription the service reports gone.

**The VAPID key is a `Core\Crypto\KeyPair` of kind `P256`.** It is `secret`, and the token is signed by
the code `Core\Jwt` already uses for ES256 (`crates/nvs-stdlib/src/jwt.rs:1333`).

**Trade-offs.** A web application gains push notifications with no package and no JavaScript library on
the server. Latency: one key agreement, one HKDF, one signature and one outbound request per message,
paid only by the request that sends it. Memory: one message, per call. Simplicity: one class with a
small surface. A caller cannot choose the cipher, the nonce or the record size, and the cipher roster
stays as it is.

## Stages

### 1. a payload is encrypted as RFC 8291's `aes128gcm` encoding, with the nonce derived inside

**Does:** a payload becomes one `aes128gcm` record for the subscription's keys, with a fresh salt and
key pair per message.

- [ ] The key agreement with the subscription's `p256dh`, the HKDF steps with its `auth` secret, and the content key and nonce derived from them (`rule:core-classes/web-push-sends-an-encrypted-message`). `crates/nvs-stdlib/src/webpush.rs:1`, `crates/nvs-stdlib/src/crypto.rs:1808`, `crates/nvs-stdlib/src/crypto.rs:1748`
- [ ] AES-128-GCM from the `aes-gcm` crate, used only inside this file, with the padding delimiter and the record header. `crates/nvs-stdlib/src/webpush.rs:1`, `crates/nvs-stdlib/src/crypto.rs:1601`
- [ ] A test that encrypts RFC 8291's example message with its fixed salt and key pair and matches its published bytes, and a payload over one record's size throws. `crates/nvs-stdlib/src/webpush.rs:1`

### 2. a VAPID token is signed with the application's key, and the message is sent under `net.connect`

**Does:** the message carries a signed VAPID token for the endpoint's origin and is sent through
`Core`'s HTTP client.

- [ ] The VAPID token: audience from the endpoint's origin, an expiry, the subject the application passes, signed ES256 with the application's key (RFC 8292). `crates/nvs-stdlib/src/webpush.rs:1`, `crates/nvs-stdlib/src/crypto.rs:2977`, `crates/nvs-stdlib/src/jwt.rs:1333`
- [ ] The request goes through the HTTP client under `net.connect`, with the outbound address table applied, and the push service's status is returned. `crates/nvs-stdlib/src/http.rs:461`
- [ ] A test against a push service on a loopback listener that decrypts what it receives and checks the token. `crates/nvs-stdlib/src/webpush.rs:1`

### 3. the subscription is tainted, the key is secret, and the class is on the protocol roster

**Does:** what the browser sent is `tainted`, the VAPID key stays `secret`, and the class meets the
roster's admission test.

- [ ] The subscription's endpoint and keys are `tainted`, and the VAPID key is `secret` and never appears in a message or an error. `crates/nvs-stdlib/src/webpush.rs:1`, `crates/nvs-stdlib/src/taint.rs:1`
- [ ] A `p256dh` value not on the curve, an `auth` value of the wrong length and an endpoint that is not `https:` each throw. `crates/nvs-stdlib/src/webpush.rs:1`
- [ ] The class is on `rule:security/protocol-roster` and passes `rule:security/protocol-admission-test`. `data/rules/security/protocol-roster.json:1`

### 4. `Core\WebPush` has its feature proofs, and the rule is shipped

**Does:** each member has its about page, tests, examples, bench, attack and reference card.

- [ ] Rust tests for each refusal, and conformance cases for each error a program can catch. `crates/nvs-stdlib/src/webpush.rs:1`
- [ ] `bun nv proofs --group 'Core\WebPush'` owes nothing; the examples show the browser's subscription beside the server's send, in the shape of `Core\Totp::check`'s. `docs/examples/core/Totp/check/03-a-login-form.nvs:1`
- [ ] The attack: a subscription whose endpoint names an internal address does not reach it, in the shape of `Core\Totp::check`'s. `tests/hostile/core/Totp/check/01-guess-every-code.nvs:1`
- [ ] The rule is `shipped`, and `bun nv owners --closes core-web-push` passes. `data/rules/core-classes/web-push-sends-an-encrypted-message.json:1`

## Handoff

### State

The goal is new. No stage is started. `Core\Passkey` (goal `core-passkey`) is the nearest class in
shape, and `Core\Crypto` has every primitive but AES-128-GCM.

### Next

Stage 1, the `aes128gcm` encoding:
- the key agreement and HKDF steps (`rule:core-classes/web-push-sends-an-encrypted-message`). `crates/nvs-stdlib/src/crypto.rs:1808`
- AES-128-GCM inside the new file, and RFC 8291's example as a test. `crates/nvs-stdlib/src/webpush.rs:1`

### Backlog

- The registry's owner notifications and its "I am here" reminder send through this class in goal `registry-service` (`docs/goals/registry-service.md`).
