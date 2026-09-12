# Handoff

## State

**Goal `webcrypto` — Novis reads what a browser encrypts and what an issuer signs. Stage 3 is half
landed: the dependency graph and the first three primitives, all crate-private.** Goal
`template-format`'s list is this goal's Stage 1 floor and is green; Stage 2 is
[ADR 0179](../decisions/0179.md), which is the design and the one home for it.

On disk in `crates/nvs-stdlib/src/crypto.rs`: `gcm_cipher`/`gcm_seal_under`/`gcm_open_under` under
the GCM specification's Appendix B cases 13–16, `pbkdf2_sha256`/`derive_key` under RFC 7914 § 11 and
`expand_key` under RFC 5869 A.1–A.3. No `Core` member is registered for any of them — that is stage
4 — so each carries `#[cfg_attr(not(test), expect(dead_code, …))]`, which fails the day stage 4
lands and is how the marker deletes itself.

The manifest half is done and does not need revisiting: `aes-gcm`, `aes-kw`, `p256`, `x25519-dalek`,
`hkdf`, `pbkdf2`, the `sha2-v11` row the playbook bullet explains, and `ring` reaching `nvs-stdlib`
directly with its recorded C-dependency answer widened from TLS to JWS. Nothing is blocked.

## Next group

**Stage 3: the key half, under their published vectors** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, appended after `expand_key` at
`crates/nvs-stdlib/src/crypto.rs:661`, with the tests beside the three that are already there.

- [ ] **Key agreement, crate-private: X25519 and ECDH over P-256** — appended at
      `crates/nvs-stdlib/src/crypto.rs:661`, each taking raw scalars and a public point and answering
      the shared secret `expand_key` is fed. An all-zero X25519 secret is refused and a P-256 point
      off the curve, the point at infinity and a short encoding are refused where the point is read,
      which is `rule:core-classes/crypto-interop-tier` and the goal's § *Standing decisions* under
      *Keys*. RFC 7748 § 6.1 is X25519's vector.
- [ ] **AES Key Wrap and Concat KDF, internal only and never a member** — appended at
      `crates/nvs-stdlib/src/crypto.rs:661`, the two
      pieces `rule:security/jwe-compact-subset` names as PBES2's and ECDH-ES's own. RFC 3394 § 4.1's
      256-bit-key vector and RFC 7518 Appendix C's Concat KDF output, which
      `tools/webcrypto-vectors.mjs` reproduces before it writes the set.
- [ ] **The four signature algorithms through `ring`** — `Crypto::sign`/`verify`'s primitives,
      appended at `crates/nvs-stdlib/src/crypto.rs:661`:
      RSASSA-PKCS1-v1_5 and RSASSA-PSS over SHA-256, ECDSA P-256 as the 64-octet
      `r ‖ s` and never DER, and Ed25519, with the algorithm coming from the key
      (`rule:security/algorithm-comes-from-the-key`) and a verdict that throws rather than answers a
      `bool` (`rule:security/verification-throws-and-compares-in-constant-time`). Confirm here
      whether `ring` takes randomness from its caller for ES256 and PS256 and say so in the module
      doc — the backlog has carried that as not checked.

## Backlog

- `crates/nvs-stdlib/src/crypto.rs` is 997 lines and stage 3's remaining rows roughly double it;
  whether the key material moves to a module of its own is a decision stage 4 should make rather
  than discover — `docs/agent/playbook.md` § *Splitting a file that got too big*.
- Stage 3's last row, the JWK thumbprint — `docs/agent/loop-goal.md` § *Stage 3* has its vector.
- Structured claims under a *shared* key stay out of this goal — ADR 0179 § *Consequences*.
- JWE-encrypted ID tokens and RSA-OAEP with them, skipped by the user — ADR 0179 § *Alternatives
  rejected*.
- `Core\Signature` is still the roster's one entry with no member, module or case —
  `rule:core-classes/signature`.
