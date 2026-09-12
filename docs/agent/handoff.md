# Handoff

## State

**Goal `webcrypto`, stage 6: `Core\Jwt\KeySet` is on disk and registered.** One member,
`read(tainted string $jwks, {rsaScheme?})` (`crates/nvs-stdlib/src/jwt.rs:1316`),
applies every admission rule the goal's § *Standing decisions* and ADR 0179 § 7 name, so nothing
downstream asks a question about a key again. A key this roster has no use for is **skipped** and a
document that is wrong is **refused whole** — [`crate::jwt`]'s module doc § *a key set is an
admission* is the home of that split, and of what a set spends.

**The set is one slot.** `framed`/`keys_in` (`crates/nvs-stdlib/src/jwt.rs:1244`) write and read
the admitted keys as one `bytes`: a kind octet, then the `kid` and the `SubjectPublicKeyInfo`, each
behind a four-octet length. `keys_in` is written, unit-tested and `expect(dead_code)` until
`verifyIssued` reads it. Nothing is registered that reads a key back out.

**The frozen set is the proof.** All eight `/jws/keySets` cases in
`crates/nvs-stdlib/tests/vectors/webcrypto.json` are admitted or refused as the script that wrote
them judged, down to which `kid`s survive — `every_frozen_key_set_is_admitted_or_refused_as_the_set_says`.

`Jwt::sign`, `Jwt::verify` and `Jwe` are untouched. The failing acceptance check
(`examples/webcrypto.nvs`) stays red until stage 7; nothing is blocked.

**A `[context]` gap:** no field points at `crates/nvs-stdlib/tests/vectors/webcrypto.json`, and its
`/jws/keySets` and `/jws/refusals` sections are the fixtures for every remaining stage-6 item. Add a
`files` selector for it, and for `crates/nvs-stdlib/src/tests/vectors.rs`, which is the door onto it.

## Next group

**Stage 6: JWS — `signObject` and `verifyIssued`** — one file set: `crates/nvs-stdlib/src/jwt.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/json.rs`, and for the last item
`crates/nvs-types/src/expr/args.rs`. The goal's § *Standing decisions* and ADR 0179 § 7 fix the whole
surface.

- [ ] **Decide the claims parameter's spelling, then write `Jwt::signObject(object $claims, Duration
      $lifetime, Crypto\KeyPair $key, {kid?, typ?, embedKey?}): string` beside
      `crates/nvs-stdlib/src/jwt.rs:248`.** `CoreTy` has no `object` (the variant list is
      `crates/nvs-stdlib/src/registry.rs:225`) although `Ty::Object` exists at
      `crates/nvs-types/src/ty.rs:104`, so it is either a new variant lowered in
      `crates/nvs-types/src/core_lib.rs:454`'s match, or `CoreTy::Mixed` as `Core\Json::encode`
      writes it (`crates/nvs-stdlib/src/json.rs:221`) with the body refusing a payload that does not
      encode as a JSON object. `Mixed` carries no classification
      (`crates/nvs-stdlib/src/registry.rs:1085`), so check what the unclassified-parameter audit and
      `rule:security/unclassified-parameter-refuses-tainted` make of it before choosing — a claims
      shape built out of a request is `tainted` and has to reach this parameter.
- [ ] **The body is `payload_of`'s twin (`crates/nvs-stdlib/src/jwt.rs:722`).** Encode with
      `crate::json::Encodable::document` (`crates/nvs-stdlib/src/json.rs:568`), which is the one
      encoder, then splice `"iat"` and `"exp"` in before the closing brace; either among the caller's
      own members is a `LogicError` as it is today, and a shared key is one too
      (`rule:security/jwt-expiry-is-mandatory`, and the goal's § *Standing decisions* for why
      structured claims are a pair's alone).
- [ ] **`Jwt::verifyIssued<T>`, beside `crates/nvs-stdlib/src/jwt.rs:920`.** The key
      is a lookup in `crates/nvs-stdlib/src/jwt.rs:1282` and never a try
      (`rule:security/algorithm-comes-from-the-key`); the header policy, the clock and the claim
      order are ADR 0179 § 7's four paragraphs, and `T` is held to
      `rule:security/derived-codec-qualifiers` at the call site in
      `crates/nvs-types/src/expr/args.rs`. `/jws/refusals` in the vector file is 34 tokens this must
      refuse, each with its `kind`.

## Backlog

- `examples/webcrypto.nvs`, the acceptance fixture, is stage 7 — `docs/agent/loop-goal.md`.
- Structured claims under a shared key stay out — the goal's § *Not this goal*.
- `Jwt\KeySet` has no accessor on purpose, so a `.nvst` case observes an admission only by the
  absence of a refusal; the record names no member that would change that.
- `PublicKey::thumbprint` (`crates/nvs-stdlib/src/crypto.rs:2010`) is still `expect(dead_code)`,
  waiting for the member that answers a `kid` nobody assigned.
