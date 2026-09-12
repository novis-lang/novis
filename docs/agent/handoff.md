# Handoff

## State

**Goal `webcrypto`, stage 7 is most of the way through.** `Core\Jwt::verifyIssued` now has a
crate-private entry point that takes the clock as an argument — `issued_claims`
(`crates/nvs-stdlib/src/jwt.rs:1676`) is all of the member but its two ends, and the arm at
`crates/nvs-stdlib/src/jwt.rs:1854` is left with the arguments, the shape question, the clock read off
`ctx` and the key lookup. The key half split with it: `found_key` is the lookup rule over a set's
frame and `key_of` turns one SPKI into a key and its algorithm, so a test finds a key by exactly the
code a program's `Core\Jwt\KeySet` answers with.

**The frozen set's whole JWS half is replayed.** All 10 `jws.vectors` verify at the second the set
judged them at and answer their payload byte for byte, and all 34 `jws.refusals` are refused with the
sentence their stage may say (`crates/nvs-stdlib/src/jwt.rs:2680`). `jws.keys` and `jws.keySets` were
already on disk — nothing was owed there.

**One thing the set and this module classify differently, and neither is wrong.** The set records
`claims` for a token carrying no numeric `exp`; this module answers it with the expiry error rather
than with `claim_refused`, because a token with no `exp` is `rule:security/jwt-expiry-is-mandatory`
rather than a claim the two parties disagree about. The refusal test says so where it asserts it.

Still red, and expected: `examples/webcrypto.nvs`, which the goal's § *Stage 7* puts last.

## Next group

**Stage 7: the primitives the frozen set records** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, with `crates/nvs-stdlib/src/tests/vectors.rs` and
`crates/nvs-stdlib/tests/vectors/webcrypto.json` read-only. The goal's § *Stage 7* first two bullets
are the list; `webcrypto::vectors("<section>")` and `webcrypto::refusals("<section>")` are the door,
and `crates/nvs-stdlib/src/jwt.rs:2680` is the replay shape to follow.

- [ ] **Replay `aesGcm`'s 7 vectors and 5 refusals in `crates/nvs-stdlib/src/crypto.rs:1565`'s own
      tests.** `gcm_seal_under` handed the recorded nonce answers `sealed` byte for byte, and
      `gcm_open_under` (`crates/nvs-stdlib/src/crypto.rs:1608`) answers the plaintext; every refusal
      is the one forgery `RuntimeError`, which `rule:security/verification-throws-and-compares-in-constant-time`
      is the home of. The layout is `nonce(12) ‖ ciphertext ‖ tag(16)`, as the set's `about` says.
- [ ] **Replay `pbkdf2`'s 2 vectors and 3 refusals against
      `crates/nvs-stdlib/src/crypto.rs:1630`.** The bounds are refused before the first HMAC in
      `derive_key` (`crates/nvs-stdlib/src/crypto.rs:1648`), and the goal's § *Standing decisions*
      fixes both numbers — a refusal at each edge is in the set, so neither may move here.
- [ ] **Replay `hkdf`'s 3 vectors against `crates/nvs-stdlib/src/crypto.rs:1694`.** RFC 5869's own
      derivations stay inline where they are asserted (`crates/nvs-stdlib/src/tests/vectors.rs:14`
      says why); these are the browser's, so they come from the file.

## Backlog

- `jws.signs`, 4 cases: `sign` at `clock` answers `token` byte for byte where `deterministic` —
  needs the same clock-as-argument split on `sign` that `verifyIssued` now has
  (docs/agent/loop-goal.md § *Stage 7*).
- `examples/webcrypto.nvs` and its `exact` check — the goal's § *Stage 7* closing paragraph, and the
  acceptance check the driver reports red every session.
- The four proofs beyond the example — an attack, a bench, a `covers:` marker for
  `Core\Jwt::verifyIssued` (`rule:testing/four-proofs`, `python tools/dossier.py --id`).
