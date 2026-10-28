---
looper: 1
title: "`Core\\Passkey` registers and verifies a passkey"
summary: "Core\\Passkey creates WebAuthn registration and authentication options and verifies both ceremonies for ES256 and Ed25519 credentials without verifying attestation, the application stores the credential it returns, the class is on the protocol roster and has its feature proofs, rule core-classes/passkey-verifies-a-webauthn-ceremony is shipped, and the checks below pass."
context: [crates/nvs-stdlib/src/passkey.rs, docs/rules/core-classes/passkey-verifies-a-webauthn-ceremony.md]
stages:
  - id: 1
    title: a registration is created and verified, and the credential is returned to the application
    status: done
    context: [crates/nvs-stdlib/src/totp.rs, crates/nvs-stdlib/src/crypto.rs, crates/nvs-stdlib/src/session.rs, crates/nvs-stdlib/src/registry.rs, Cargo.toml]
    extra:
      context:
        rules: [core-classes/passkey-verifies-a-webauthn-ceremony, packaging/somebody-elses-specification-is-a-crate, packaging/a-c-dependency-answers-two-questions, core-classes/session-is-started-explicitly, core-api/reference-card]
  - id: 2
    title: an authentication is verified against a stored credential
    status: done
    context: [crates/nvs-stdlib/src/crypto.rs]
    extra:
      context:
        rules: [core-classes/passkey-verifies-a-webauthn-ceremony, core-api/reference-card]
  - id: 3
    title: the inputs are tainted, the challenge is single-use, and the class is on the protocol roster
    status: done
    context: [crates/nvs-stdlib/src/taint.rs, docs/rules/security/protocol-roster.md, docs/rules/security/protocol-admission-test.md]
    extra:
      context:
        rules: [core-classes/passkey-verifies-a-webauthn-ceremony, security/protocol-roster, security/protocol-admission-test]
  - id: 4
    title: "the registration members have their feature proofs"
    status: done
    context: [docs/examples/core/Totp/check/about.md, tests/hostile/core/Totp/check/01-guess-every-code.nvs]
  - id: 5
    title: "the sign-in members have their feature proofs, and the rule is shipped"
    status: done
    context: [docs/examples/core/Totp/check/about.md, tests/hostile/core/Totp/check/01-guess-every-code.nvs]
  - id: 6
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: passkey-proofs
    stage: 5
    name: Core\Passkey owes no feature proof
    run: [bun, nv, proofs, --gate, --group, "Core\\Passkey"]
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, core-passkey]
    reach: true
    floor: false
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
`rule:core-classes/passkey-verifies-a-webauthn-ceremony` states**: `registerOptions`,
`verifyRegistration`, `signInOptions`, `credentialId` and `verifySignIn`, with the value
`Passkey\Credential` and the enum `Passkey\Credential::Algorithm`. Each member's reference card is its registry
declaration (`rule:core-api/reference-card`), written with the member's code, as `Core\Totp`'s is
(`crates/nvs-stdlib/src/totp.rs:132`).

**The credential is a `Core` instance with four readers and a static `of`.** A `Core` instance has no
property a program can reach, so `id()`, `publicKey()`, `algorithm()` and `signCount()` read its four
slots, and `Passkey\Credential::of($id, $publicKey, $algorithm, $signCount)` builds a stored one
again, checking the COSE key against the algorithm. The enum is `Passkey\Credential::Algorithm`,
because the credential is the one class that takes or returns it
(`rule:enums/an-enum-lives-in-the-class-that-uses-it`, held by `crates/nvs-stdlib/tests/enum_owners.rs`).
Why: without `of`, a credential the application stored could never reach `verifySignIn`.

**The challenge is saved with the second it was issued, and expires after five minutes.** The
options send `timeout: 300000`, and a challenge older than that throws, so a ceremony the browser
abandoned cannot be completed later. Memory: 40 bytes per ceremony in flight, in the session.
`crate::session::put_bytes` and `take_bytes` (`crates/nvs-stdlib/src/session.rs`) are the two doors
from Rust into the record.

**`registerOptions`' text and bytes arguments are `Contagious`**, because each is copied into the
JSON it returns, and `verifyRegistration`'s three are `Neutral`, because the answer is a checked
object.

**ES256 and Ed25519 only, and no attestation verification.** A credential with another algorithm is
refused at registration. The attestation statement is not checked, so no vendor certificate list is
shipped or updated.

**The application stores credentials.** `Core\Passkey` reads and writes no storage. Registration
returns a credential value (its id, its public key, its algorithm and its sign counter), and
authentication takes that value and returns it with the counter updated. The application saves it
wherever it keeps users.

**The challenge is created by `Core\Passkey`, comes from the system random source, and is accepted
once.** It is saved in the started session (`rule:core-classes/session-is-started-explicitly`) and
taken out before verification checks anything. The origin and the relying party id are checked
against values the application passes, never against anything the browser sent.

**No capability row.** The class reaches no file, socket or process, so it is not capability-bearing
and adds no row to `registry::CAPABILITIES` (`crates/nvs-stdlib/src/registry.rs:1809`); goal
`namespace-grants`'s check never asks a grant for it.

**The CBOR and COSE parsing is `coset`, on the `ciborium` it depends on**, both pure Rust, chosen under
`rule:packaging/somebody-elses-specification-is-a-crate` and
`rule:packaging/a-c-dependency-answers-two-questions`. `ciborium` reads the attestation object and
`coset` the credential's COSE key. If `cargo deny` refuses `coset`, `ciborium` alone reads the COSE key
by its labels. Signatures are verified by the code `Core\Crypto` already uses
(`crates/nvs-stdlib/src/crypto.rs:2642`).

**A software authenticator in the test module makes the fixtures.** It generates an ES256 and an
Ed25519 key with `Core\Crypto`'s code, builds the client data, the authenticator data and a `none`
attestation object with `ciborium`, and signs. WebAuthn Level 3's published test vectors are added as
fixed bytes for each algorithm the specification gives one for, as an independent check. Conformance
cases build the whole ceremony in Novis: the client data from the options' challenge, the
authenticator data and the `none` attestation object from hex and `Core\Bytes`, an Ed25519 COSE key
around `Core\Crypto`'s raw public key, and the assertion signed with `Core\Crypto::sign`. No fixed
attestation object is embedded, so one fresh key registers and then signs in. Why: no browser runs
in a test, and a hand-copied ceremony could not carry a fresh challenge.

**Trade-offs.** A web application gains passwordless sign-in with no package and no JavaScript library
on the server. Latency: one signature check per sign-in, on the request that signs in. Memory: the
challenge, per request, in the session. Simplicity: one class with a small surface; the browser half
stays the browser's `navigator.credentials` API, which the examples show. Two crates join the
workspace for one class.

**Lanes.** The whole goal can run as a side goal beside the chain now: it adds
`crates/nvs-stdlib/src/passkey.rs`. Stages 1 and 2 share `passkey.rs` and `crypto.rs` and run in order,
in one group when both fit. Stage 3 follows, then 4 and 5. It shares the class list in
`crates/nvs-stdlib/src/registry.rs:5994`, the module list in `crates/nvs-stdlib/src/lib.rs:230`, the
workspace `Cargo.toml`, `docs/rules/security/protocol-roster.md` and
`data/rules/security/protocol-roster.json` with goal `core-web-push`; whichever lands second rebases
over those lines.

## Stages

### 1. a registration is created and verified, and the credential is returned to the application

**Does:** the class creates registration options and verifies the browser's response, returning a
credential value.

**Split:** first, an `Explore` subagent returns how a `Core` member reads and writes the started
session from Rust (`file.rs:NN`, one line each), starting at `crates/nvs-stdlib/src/session.rs:617`.

- [x] `coset` and `ciborium` in the workspace `Cargo.toml` and `crates/nvs-stdlib/Cargo.toml`, each row saying why, with `cargo deny` green (`rule:packaging/somebody-elses-specification-is-a-crate`). `Cargo.toml:1`
- [x] The class, its module and its place in the class list. `crates/nvs-stdlib/src/passkey.rs:1`, `crates/nvs-stdlib/src/lib.rs:230`, `crates/nvs-stdlib/src/registry.rs:5994`
- [x] `registerOptions`: the JSON for `navigator.credentials.create()`, with a fresh challenge saved in the session. `crates/nvs-stdlib/src/passkey.rs:1`
- [x] `verifyRegistration`: the client data (type, challenge, origin), the authenticator data (relying party id hash, user present, user verified), the COSE key for ES256 and Ed25519 with any other algorithm refused, and the `Passkey\Credential` returned (`rule:core-classes/passkey-verifies-a-webauthn-ceremony`). `crates/nvs-stdlib/src/passkey.rs:1`
- [x] The reference cards of `registerOptions`, `verifyRegistration`, `Passkey\Credential` and `Passkey\Credential::Algorithm` (`rule:core-api/reference-card`). `crates/nvs-stdlib/src/passkey.rs:1`
- [x] Rust tests: the software authenticator's registration for each algorithm, the spec's vectors, and a third algorithm refused. `crates/nvs-stdlib/src/passkey.rs:1`

### 2. an authentication is verified against a stored credential

**Does:** the browser's assertion is verified against the credential the application stored, and the
updated credential is returned.

- [x] `signInOptions` and `credentialId`. `crates/nvs-stdlib/src/passkey.rs:1`
- [x] `verifySignIn`: the assertion's client data, authenticator data and signature over both. `crates/nvs-stdlib/src/passkey.rs:1`, `crates/nvs-stdlib/src/crypto.rs:2642`
- [x] A sign counter that does not increase, when either value is not zero, throws. `crates/nvs-stdlib/src/passkey.rs:1`
- [x] The reference cards of `signInOptions`, `credentialId` and `verifySignIn` (`rule:core-api/reference-card`). `crates/nvs-stdlib/src/passkey.rs:1`
- [x] Rust tests: the software authenticator's sign-in for each algorithm, a wrong signature, and a counter that goes back. `crates/nvs-stdlib/src/passkey.rs:1`

### 3. the inputs are tainted, the challenge is single-use, and the class is on the protocol roster

**Does:** what the browser sent is `tainted`, a challenge verifies once, and the class meets the
roster's admission test.

- [x] The browser's response is `tainted` in each member's declaration, `credentialId` returns `tainted bytes`, and the verified credential is not tainted. `crates/nvs-stdlib/src/passkey.rs:1`, `crates/nvs-stdlib/src/registry.rs:185`
- [x] A challenge used twice, a wrong origin and a wrong relying party id each throw. `crates/nvs-stdlib/src/passkey.rs:1`
- [x] Conformance cases for each refusal, built as the standing decision says. `tests/conformance/core/passkey-a-challenge-verifies-once-and-only-for-its-origin.nvst:1`
- [x] `rule:security/protocol-roster` names `Core\Passkey` as built, not only admitted, and its case joins `guardedBy` (`rule:security/protocol-admission-test`). `docs/rules/security/protocol-roster.md:7`, `data/rules/security/protocol-roster.json:10`

### 4. the registration members have their feature proofs

**Does:** `registerOptions`, `verifyRegistration` and `credentialId` each have their about page,
examples, bench and attack.

**Split:** one writing subagent per member, one after another, writes the about page, the three
examples (the browser half beside the server half, in the shape of `Core\Totp::check`'s) and the
bench, held to § *Text an end user reads* in `AGENTS.md`. A last one writes the attack.

- [x] `registerOptions`. `docs/examples/core/Passkey/registerOptions/about.md:1`, `benches/members/core/Passkey/registerOptions.nvs:1`
- [x] `verifyRegistration`. `docs/examples/core/Passkey/verifyRegistration/about.md:1`, `benches/members/core/Passkey/verifyRegistration.nvs:1`
- [x] `credentialId`. `docs/examples/core/Passkey/credentialId/about.md:1`, `benches/members/core/Passkey/credentialId.nvs:1`
- [x] The attack: a replayed registration. `tests/hostile/core/Passkey/verifyRegistration/01-a-replayed-registration.nvs:1`
- [x] `bun nv proofs --id` owes nothing for the three but the perf record, which stage 5 makes for all five members. `docs/examples/core/Passkey/registerOptions/about.md:1`

### 5. the sign-in members have their feature proofs, and the rule is shipped

**Does:** `signInOptions` and `verifySignIn` have their proofs, the class owes nothing, and the rule is
`shipped`.

**Split:** as stage 4, one writing subagent per member, then the attack.

- [x] `signInOptions`. `docs/examples/core/Passkey/signInOptions/about.md:1`, `benches/members/core/Passkey/signInOptions.nvs:1`
- [x] `verifySignIn`. `docs/examples/core/Passkey/verifySignIn/about.md:1`, `benches/members/core/Passkey/verifySignIn.nvs:1`
- [x] The attack: a replayed assertion. `tests/hostile/core/Passkey/verifySignIn/01-a-replayed-assertion.nvs:1`
- [x] The perf record of all five members, after the last change to `passkey.rs`: `bun nv proofs --record-perf --id <member>` for each, and `bun nv scaling --budget --write` for the new benches' shares. The budget write is refused by 45 benches of other classes over their shares; no gate runs it, and the Backlog holds it. `data/perf/members.ndjson:1`, `data/perf/bench-budget.json:1`
- [x] `bun nv proofs --group 'Core\Passkey'` owes nothing. `docs/examples/core/Passkey/verifySignIn/about.md:1`
- [x] `rule:core-classes/passkey-verifies-a-webauthn-ceremony` is `shipped` with its cases in `guardedBy`, and `bun nv owners --closes core-passkey` passes. `data/rules/core-classes/passkey-verifies-a-webauthn-ceremony.json:3`

### 6. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1 to 5 are done. All five members have an about page, three examples, a bench and an
attack, and `bun nv proofs --gate --group 'Core\Passkey'` owes nothing (`passkey-proofs` is green).
The rule is `shipped`, guarded by `passkey.rs` and its four cases, and
`bun nv owners --closes core-passkey` passes (`owners-closes` is green). The rendered
`docs/rules/security.md` now lists `Core\Passkey` as built, as its fragment already did.

The perf tool subtracted a floor taken without `--config`, while `spawnProof` runs a bench beside an
`nvs.toml` under that file and so skips the repository's root `nvs.toml`. Such a bench read about
11 ms over N too low, and `credentialId` read 0. `floorFor` (`tools/nv/proofs/perf.ts`) now times
the empty program under each bench's own config. The five Passkey figures are taken with it, and
`credentialId`'s bench runs 16384 rounds.

**Expected red:** none from this goal. The proofs of other classes whose benches sit beside an
`nvs.toml` (Session, Cache, Db and the like) still carry figures taken against the old floor. Their
`impl_hash` has not changed, so no gate goes red for it.

### Next

**Stage 6: verify the goal once.** Start `bun nv affected --run` with `run_in_background`, end the
turn, group its failures by cause, fix them in one pass, and run it again. Then the goal-end gates:
`bun nv verify --doc`, `bun nv owners --closes core-passkey`, `bun nv ci --check`. The run also
shows whether plain `bun nv proofs --run` (without `--nvs`) still fails to build its coverage
binary. `tools/nv/cmd/verify.ts:1`

- [ ] `bun nv affected --run` over the whole goal, failures grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure fixed in one pass, then a green second run. `tools/nv/cmd/verify.ts:1`
- [ ] The goal-end gates, each red fixed. `tools/nv/cmd/ci.ts:1`

### Backlog

- The registry's sign-in uses this goal's work in goal `registry-service` (`docs/goals/registry-service.md`).
- Web Push, with its AES-128-GCM kept inside the class, is goal `core-web-push`'s (`docs/goals/core-web-push.md`).
- `bun nv proofs --run` without `--nvs` failed to build its coverage binary under `target/covws` in
  stage 4. The cause is not known. Stage 6 runs it, and that run shows whether it is still broken.
  `tools/nv/proofs/run.ts:1`
- `bun nv scaling --budget --write` is refused by 45 benches over their shares: the Spreadsheet
  package's (by 1 or 32 statements), `Core\Metrics`, `Core\Cap::has` and the pipeline operator. No
  gate runs the budget, so the Passkey benches have no share yet. Whoever owns the budget lowers
  those N or writes their reasons, then `--write` adds the Passkey shares. `tools/nv/cmd/scaling.ts:1300`
- Every bench beside an `nvs.toml` outside `Core\Passkey` has a figure taken against the old floor,
  so it reads low. A `--record-perf --force` over those groups corrects them. `tools/nv/proofs/perf.ts:floorFor`
- A `foreach` over an inline array literal, `foreach (['a', 'b'] as string $email)`, fails with
  E0401 "expected `string`, found `mixed`". The same array in an `array<string>` variable works.
  This is a checker question for whichever goal owns array literal inference.
