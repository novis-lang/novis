---
looper: 1
title: "`Core\\WebPush` sends an encrypted, signed Web Push message"
summary: "Core\\WebPush encrypts a payload for a browser's push subscription as RFC 8291's aes128gcm encoding with the nonce derived inside and never a parameter, signs a VAPID token with the application's P-256 key as RFC 8292 says, and sends the message through Core's HTTP client under the ordinary net.connect grant, AES-128-GCM stays inside the class and is not a case of Core's cipher roster, the class is on the protocol roster, it has its feature proofs, rule core-classes/web-push-sends-an-encrypted-message is shipped, and the checks below pass."
context: [crates/nvs-stdlib/src/webpush.rs, docs/rules/core-classes/web-push-sends-an-encrypted-message.md]
stages:
  - id: 1
    title: a payload is encrypted as RFC 8291's `aes128gcm` encoding, with the nonce derived inside
    status: done
    context: [crates/nvs-stdlib/src/crypto.rs, crates/nvs-stdlib/src/registry.rs, Cargo.toml]
    extra:
      context:
        rules: [core-classes/web-push-sends-an-encrypted-message, core-classes/crypto-interop-tier, core-api/reference-card]
  - id: 2
    title: a VAPID token is signed with the application's key, and the message is sent under `net.connect`
    status: done
    context: [crates/nvs-stdlib/src/crypto.rs, crates/nvs-stdlib/src/jwt.rs, crates/nvs-stdlib/src/http.rs, crates/nvs-stdlib/src/http/transport.rs]
    extra:
      context:
        rules: [core-classes/web-push-sends-an-encrypted-message, security/capability-check-at-the-door, testing/capability-completeness-test, security/net-address-policy]
  - id: 3
    title: the subscription is tainted, the key is secret, and the class is on the protocol roster
    status: done
    context: [crates/nvs-stdlib/src/taint.rs, docs/rules/security/protocol-roster.md, docs/rules/security/protocol-admission-test.md]
    extra:
      context:
        rules: [core-classes/web-push-sends-an-encrypted-message, security/protocol-roster, security/protocol-admission-test]
  - id: 4
    title: "`Core\\WebPush` has its feature proofs, and the rule is shipped"
    status: done
    context: [docs/examples/core/Totp/check/about.md, tests/hostile/core/Totp/check/01-guess-every-code.nvs]
  - id: 5
    title: the goal is verified once, and every failure is fixed
    status: done
    gate: true
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
accepts for the wrong sender. So the whole message is built in `Core` beside `Core\Jwt` and
`Core\Totp`, and every Novis web application can notify its users without a package. P-256 agreement,
ECDSA signing and HKDF are already in `Core\Crypto` (`crates/nvs-stdlib/src/crypto.rs:1830`,
`crates/nvs-stdlib/src/crypto.rs:2999`, `crates/nvs-stdlib/src/crypto.rs:1770`).

## Standing decisions

**The member names and shapes are the ones
`rule:core-classes/web-push-sends-an-encrypted-message` states**: `subscription`,
`applicationServerKey` and `send`, with the values `WebPush\Subscription` and `WebPush\Sent`. The class
takes a subscription (the endpoint URL and the browser's `p256dh` and `auth` values), a payload and the
application's VAPID key, and sends one message. Each member's reference card is its registry
declaration (`rule:core-api/reference-card`), written with its code.

**The nonce is derived inside and is never a parameter.** Each message draws a fresh salt and a fresh
P-256 key pair from the system random source, and the content key and the nonce come from HKDF as
RFC 8291 says. No `Core` cipher takes a caller-chosen nonce, and this class adds none.

**AES-128-GCM is internal to `Core\WebPush`.** It comes from the `aes-gcm` crate that already provides
AES-256-GCM (`crates/nvs-stdlib/src/crypto.rs:1623`); its `aes` feature (`Cargo.toml:766`) already
gives `Aes128Gcm`, so no row changes. It is not a new case of `Core\Crypto\Cipher`, so
`rule:core-classes/crypto-interop-tier`'s roster does not change.

**The message is sent through `Core`'s HTTP client under the ordinary `net.connect` grant.** `send`
takes the transport path `extension_send` takes (`crates/nvs-stdlib/src/http.rs:3980`) through a
sibling that names `Core\WebPush::send` in a refusal. The endpoint comes from a browser, so it is
`tainted`, and the client's address policy refuses an internal host as it does for any request
(`rule:security/net-address-policy`). The push service's answer is returned, so the application can
drop a subscription the service reports gone.

**`send` is a door, so the class has capability rows.** `registry::CAPABILITIES`
(`crates/nvs-stdlib/src/registry.rs:1809`) gets `send` as `net.connect` and `subscription` and
`applicationServerKey` as `None`, because the completeness test wants a row for every member of a
capability-bearing class (`crates/nvs-stdlib/tests/capability.rs:290`). Goal `namespace-grants` reads
that row to check a call site's grant, and its test checks the row against the door.

**The endpoint is always `https:`, in tests too.** The stage 2 test runs the push service as a
loopback TLS origin with a test certificate, through `tls_origin`
(`crates/nvs-stdlib/src/http/transport.rs:5499`) and a roots file, as
`https_call_through_the_client_reaches_a_loopback_origin_under_a_roots_file` does
(`crates/nvs-stdlib/src/http/transport.rs:5636`). Loopback is reached only through the operator
exception the test writes. Why: security first, so the class has no test-only escape, and the test
exercises the same TLS and address checks a real push service gets.

**`WebPush\Subscription` has one reader, `endpoint(): tainted string`.** The application stores the
browser's JSON text and reads it again with `subscription` before it sends, so the keys need no
readers, and the endpoint is the one value it looks a subscription up by. It is `tainted` because a
browser wrote it. `subscription` checks every value when it reads, so `send` trusts the slots.

**The VAPID key is a `Core\Crypto\KeyPair` of kind `P256`.** It is `secret`, and the token is signed by
the code `Core\Jwt` already uses for ES256 (`crates/nvs-stdlib/src/jwt.rs:1162`,
`crates/nvs-stdlib/src/jwt.rs:1351`).

**Trade-offs.** A web application gains push notifications with no package and no JavaScript library on
the server. Latency: one key agreement, one HKDF, one signature and one outbound request per message,
paid only by the request that sends it. Memory: one message, per call. Simplicity: one class with a
small surface. A caller cannot choose the cipher, the nonce or the record size, and the cipher roster
stays as it is.

**Lanes.** The whole goal can run as a side goal beside the chain now: it adds
`crates/nvs-stdlib/src/webpush.rs`. Its stages share `webpush.rs` and run in order. It is disjoint from
goal `core-passkey` except the class list and `CAPABILITIES` in `crates/nvs-stdlib/src/registry.rs`, the
module list in `crates/nvs-stdlib/src/lib.rs:230`, `docs/rules/security/protocol-roster.md` and
`data/rules/security/protocol-roster.json`; whichever lands second rebases over those lines.

## Stages

### 1. a payload is encrypted as RFC 8291's `aes128gcm` encoding, with the nonce derived inside

**Does:** a payload becomes one `aes128gcm` record for the subscription's keys, with a fresh salt and
key pair per message.

- [x] The class, its module, its place in the class list, and `subscription` reading `PushSubscription.toJSON()`. `crates/nvs-stdlib/src/webpush.rs:1`, `crates/nvs-stdlib/src/lib.rs:230`, `crates/nvs-stdlib/src/registry.rs:5994`
- [x] The key agreement with the subscription's `p256dh`, the HKDF steps with its `auth` secret, and the content key and nonce derived from them (`rule:core-classes/web-push-sends-an-encrypted-message`). `crates/nvs-stdlib/src/webpush.rs:1`, `crates/nvs-stdlib/src/crypto.rs:1816`, `crates/nvs-stdlib/src/crypto.rs:1830`, `crates/nvs-stdlib/src/crypto.rs:1770`
- [x] AES-128-GCM from the `aes-gcm` crate, used only inside this file, with the padding delimiter and the record header. `crates/nvs-stdlib/src/webpush.rs:1`, `crates/nvs-stdlib/src/crypto.rs:1623`
- [x] The reference cards of `subscription` and `WebPush\Subscription` (`rule:core-api/reference-card`). `crates/nvs-stdlib/src/webpush.rs:1`
- [x] Tests: RFC 8291's example message with its fixed salt and key pair matches its published bytes, and a payload over 3993 bytes throws. `crates/nvs-stdlib/src/webpush.rs:1`

### 2. a VAPID token is signed with the application's key, and the message is sent under `net.connect`

**Does:** the message carries a signed VAPID token for the endpoint's origin and is sent through
`Core`'s HTTP client.

- [x] `applicationServerKey`, and the VAPID token: audience from the endpoint's origin, an expiry, the subject the application passes, signed ES256 with the application's key (RFC 8292). `crates/nvs-stdlib/src/webpush.rs:1`, `crates/nvs-stdlib/src/jwt.rs:1351`
- [x] `send` through the transport path as the standing decision says, under `net.connect` with the address policy applied, returning `WebPush\Sent` with `status` and `gone`. `crates/nvs-stdlib/src/http.rs:3980`
- [x] The three rows in `registry::CAPABILITIES` (`rule:testing/capability-completeness-test`, `rule:security/capability-check-at-the-door`). `crates/nvs-stdlib/src/registry.rs:1809`
- [x] The reference cards of `applicationServerKey`, `send` and `WebPush\Sent` (`rule:core-api/reference-card`). `crates/nvs-stdlib/src/webpush.rs:1`
- [x] Tests: a loopback TLS push service, as the standing decision says, decrypts what it receives and checks the token; a `410` answer gives `gone`. `crates/nvs-stdlib/src/webpush.rs:1`, `crates/nvs-stdlib/src/http/transport.rs:5499`

### 3. the subscription is tainted, the key is secret, and the class is on the protocol roster

**Does:** what the browser sent is `tainted`, the VAPID key stays `secret`, and the class meets the
roster's admission test.

- [x] The subscription's endpoint and keys are `tainted`, and the VAPID key is `secret` and never appears in a message or an error. `crates/nvs-stdlib/src/webpush.rs:1`, `crates/nvs-stdlib/src/registry.rs:185`
- [x] A `p256dh` value not on the curve, an `auth` value of the wrong length and an endpoint that is not `https:` each throw. `crates/nvs-stdlib/src/webpush.rs:1`
- [x] Rust tests for each refusal, and conformance cases for each error a program can catch. `tests/conformance/core/web-push-a-subscription-that-is-not-a-browsers-throws.nvst:1`
- [x] `rule:security/protocol-roster` names `Core\WebPush` as built, not only admitted, and its case joins `guardedBy` (`rule:security/protocol-admission-test`). `docs/rules/security/protocol-roster.md:7`, `data/rules/security/protocol-roster.json:10`

### 4. `Core\WebPush` has its feature proofs, and the rule is shipped

**Does:** each member has its about page, examples, bench and attack.

**Split:** one writing subagent per member, one after another, writes the about page, the three
examples (the browser's subscription beside the server's send, in the shape of `Core\Totp::check`'s)
and the bench, held to § *Text an end user reads* in `AGENTS.md`. A last one writes the attack.

- [x] `subscription`. `docs/examples/core/WebPush/subscription/about.md:1`, `benches/members/core/WebPush/subscription.nvs:1`
- [x] `applicationServerKey`. `docs/examples/core/WebPush/applicationServerKey/about.md:1`, `benches/members/core/WebPush/applicationServerKey.nvs:1`
- [x] `send`. `docs/examples/core/WebPush/send/about.md:1`, `benches/members/core/WebPush/send.nvs:1`
- [x] The attack: a subscription whose endpoint names an internal address does not reach it. `tests/hostile/core/WebPush/send/01-an-endpoint-on-the-internal-network.nvs:1`
- [x] `bun nv proofs --group 'Core\WebPush'` owes nothing. `docs/examples/core/WebPush/send/about.md:1`
- [x] `rule:core-classes/web-push-sends-an-encrypted-message` is `shipped` with its cases in `guardedBy`, and `bun nv owners --closes core-web-push` passes. `data/rules/core-classes/web-push-sends-an-encrypted-message.json:3`

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [x] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [x] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [x] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Every stage is done, and the goal is reached. `looper verify` is green, and so are
`bun nv verify --doc`, `bun nv owners --closes core-web-push`, `bun nv ci --check`, and both
acceptance checks, `web-push-proofs` and `owners-closes`.

The gate stage fixed five causes:
- **`push_send` skipped the test's answer table.** `rule:testing/an-outbound-call-is-answered-from-a-table`
  says every outbound call is answered once a row is registered, and `Core\WebPush::send` reached the
  network instead. `crates/nvs-stdlib/src/http.rs` `push_send` now judges the endpoint and headers,
  records the message for `Core\Test::sentHttp`, and returns the row's status. `faked` and it share
  `unanswered`. `a_test_s_answer_table_answers_send_and_records_the_message` proves it.
- **`Core\WebPush\Subscription` and `Core\WebPush\Sent` owed every proof** (the inner-group playbook
  trap). Their members have about pages, examples, benches, attacks and perf rows under
  `WebPush-Subscription/` and `WebPush-Sent/`, built on the answer table.
- **The conformance floor of three** (`crates/nvs-stdlib/tests/conformance_coverage.rs`): three new
  `web-push-*.nvst` cases ask each member a new question.
- **rustfmt, the rendered rule pages, three unformatted `.nvs` files and the website data**
  (`core.json`, `llms-full.txt`, and nine `questions.json` entries).
- **`Core\Http`'s perf figures** went stale with `http.rs`, and each group was recorded again.

The `env-secret-proofs` sweep red was stale: the command exits 0 and no check by that name exists.
The cancelled nightly run 40662443492 was a dispatch that the next push's run cancelled. That run
and the one after it are green.

### Next

Nothing in this goal. The chain goes on with `registry-service`. No test is expected to go red.

### Backlog

- The registry's owner notifications and its "I am here" reminder send through this class in goal `registry-service` (`docs/goals/registry-service.md`).
- `extension_send` (`crates/nvs-stdlib/src/http.rs`) also skips the test's answer table, as `push_send` did. Whether an extension's call in a test should be answered from it is open and has no owner yet.
