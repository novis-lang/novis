# Handoff

## State

**Goal `webcrypto` — stage 3 is on disk but for one row of its own table.** Every AEAD, derivation,
agreement, wrap and now every signature algorithm the roster names is a crate-private function in
`crates/nvs-stdlib/src/crypto.rs`, each asserted against published vectors. The **JWK thumbprint** is
the stage-3 row nothing implements yet, and it is the third item below.

The signature half is `VerifyingKey`/`verify_signature` and `SignatureKind`/`SigningKey`/
`read_signing_key`/`sign`. The key's material lives *inside* the variant naming its algorithm, so
`rule:security/algorithm-comes-from-the-key` holds by construction and `SignatureKind` at
`read_signing_key` is the single door where a choice is left — `RS256` and `PS256` being one key type
and two algorithms. All of it still carries `#[cfg_attr(not(test), expect(dead_code, …))]`, which fails
the day stage 4 registers the members and is how the marker deletes itself.

`crates/nvs-stdlib/src/tests/vectors.rs` gained `node(pointer)`: the door onto what a case *names*
rather than carries, which is how a signature vector reaches the key it was made under. Nothing is
blocked.

## Next group

**Stage 4: `seal` and `open` take their cipher, and every call site moves with them** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, `examples/crypto.nvs` and the six
`tests/conformance/core/crypto-*.nvst` cases.

- [ ] **The two rows take a required `Crypto\Cipher`** — `crates/nvs-stdlib/src/crypto.rs:363` and
      `crates/nvs-stdlib/src/crypto.rs:379` gain the third parameter as
      `CoreTy::Enum(r"Crypto\Cipher")` with `defaults: &[]`, and `crates/nvs-stdlib/src/crypto.rs:1156`'s
      `keyed` splits by cipher so `gcm_seal_under` and `gcm_open_under` become reachable from a member
      rather than from tests alone. `Core\Order` at `crates/nvs-stdlib/src/arr.rs:2152` is this tree's
      precedent for a closed enum argument, and the goal's § *Standing decisions* fixes that no
      algorithm argument carries a default.
- [ ] **Every existing call moves in the same commit, so the tree never holds a red case** —
      `examples/crypto.nvs:53` onward passes `Cipher::XChaCha20Poly1305` with its frozen output
      unchanged, and each `seal`/`open` call in the six cases takes the cipher with expected output
      unchanged. `tests/conformance/core/crypto-seals-and-opens-with-no-cipher-argument.nvst:1` is
      renamed for what it now proves — no floor names it by path — and a reject case proves the old
      two-argument call no longer compiles (`rule:core-api/shape-rules`).
- [ ] **The JWK thumbprint, stage 3's one unbuilt row** — appended beside the signature seams at
      `crates/nvs-stdlib/src/crypto.rs:1154`: RFC 7638 § 3.1, which is base64url of SHA-256 over the
      required members sorted with no whitespace. The frozen set already holds both halves —
      `/jws/keys/<id>/jwkMinimal` is that canonical form and `/jws/keys/<id>/thumbprint` is the answer —
      so the primitive hashes canonical text and the assembly of that text lands with
      `PublicKey::write` (`rule:security/protocol-roster`).

## Backlog

- The stage-3 table names RFC 7515 A.2/A.3, RFC 7520 § 4.2 and RFC 8037 A.4 for the four signature
  rows; what is asserted today is the frozen WebCrypto set alone (`docs/agent/loop-goal.md` § Stage 3).
- `examples/webcrypto.nvs`, the acceptance fixture the driver reports missing, is stage 7's
  (`docs/agent/loop-goal.md` § Stage 7).
- `Crypto\Curve` becoming `Crypto\KeyKind` with the two RSA kinds is stage 4's second half, after the
  cipher argument (`docs/agent/loop-goal.md` § Stage 4).
- PEM `PRIVATE KEY` reading for `KeyPair::read` reuses what the graph carries at
  `crates/nvs-host/src/tls.rs:121` ([ADR 0179](../decisions/0179.md) § 8).
