# Handoff

## State

**Goal `webcrypto`, stage 7: the frozen set's primitive half is replayed whole.** `aesGcm`'s 7 vectors
seal to WebCrypto's own octets under the recorded nonce and open back to their plaintext, and its 5
refusals all arrive at `gcm_open_under` as `None`
(`crates/nvs-stdlib/src/crypto.rs:3863`); `pbkdf2`'s 2 vectors derive through `derive_key` and its 3
refusals sit one step outside a bound (`crates/nvs-stdlib/src/crypto.rs:4014`); `hkdf`'s 3 expansions
answer the set's key (`crates/nvs-stdlib/src/crypto.rs:4065`). `aesKw`, `ecdh`, `jwe`, `signatures` and
`jws`'s `keys`, `keySets`, `vectors` and `refusals` were already on disk.

**A numeric field of the set has a door now.** `webcrypto::number`
(`crates/nvs-stdlib/src/tests/vectors.rs:89`) is how a count is read; the playbook bullet is why it
exists, and `clock`, `lifetime` and `deterministic` are the next fields that need it.

**What stage 7 still owes is `jws.signs` and the example**, in that order — the sign half needs the
clock-as-argument split the verify half already has, and nothing else in the set is unreplayed.

Still red, and expected: `examples/webcrypto.nvs`, which the goal's § *Stage 7* puts last.

## Next group

**Stage 7: `jws.signs`, the issuer direction** — one file set: `crates/nvs-stdlib/src/jwt.rs`, with
`crates/nvs-stdlib/src/tests/vectors.rs` and `crates/nvs-stdlib/tests/vectors/webcrypto.json`
read-only. The set's 4 cases carry `key`, `options`, `claims` or `structured`, `clock`, `lifetime`,
`deterministic`, `header` and `token`.

- [ ] **Split a crate-private clock-taking entry out of `nvs_core_jwt_sign`
      (`crates/nvs-stdlib/src/jwt.rs:1281`).** The member keeps the argument reads and
      `registered_pair` (`crates/nvs-stdlib/src/jwt.rs:1158`); the split takes the `alg`/`jwk` choice,
      `header_of`, `payload_of` and the signature (`crates/nvs-stdlib/src/jwt.rs:1296-1343`), so a test
      signs at a second it names. `issued_claims` (`crates/nvs-stdlib/src/jwt.rs:1747`) is the same
      split for the other direction and the shape to follow;
      `rule:security/algorithm-comes-from-the-key` is what the `alg`/`jwk` choice keeps.
- [ ] **Replay the 3 deterministic `jws.signs` cases byte for byte at their `clock`**, in
      `crates/nvs-stdlib/src/jwt.rs`'s own tests beside `crates/nvs-stdlib/src/jwt.rs:2693`, which is
      the replay shape. Two are client assertions under RS256 and EdDSA; the third is a signed request
      object whose payload is `structured`, so it goes through the structured-claims member the registry
      row at `crates/nvs-stdlib/src/jwt.rs:573` describes. Assert `header` too, not only `token` — a
      header written non-canonically still produces a token that verifies.
- [ ] **Replay the ES256 DPoP proof, whose `deterministic` is false**, at
      `crates/nvs-stdlib/src/jwt.rs:2693`'s shape: the same header and payload segments as the set's
      `token`, and a signature that verifies under the pair's public half. Its `options` are
      `{typ: 'dpop+jwt', embedKey: true}`, so it also pins `embedded_jwk`
      (`crates/nvs-stdlib/src/jwt.rs:1313`).

## Backlog

- `examples/webcrypto.nvs` and its `exact` check — the goal's § *Stage 7* closing paragraph, and the
  acceptance check the driver reports red every session.
- The four proofs beyond the example — an attack, a bench, a `covers:` marker for
  `Core\Jwt::verifyIssued` (`rule:testing/four-proofs`, `python tools/dossier.py --id`).
