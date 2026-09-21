# Handoff

## State

Goal `Core\Crypto and 2 more` (15 features). Twelve are done and owe nothing: the six sealing and
agreement members, `Core\Crypto::generateKeyPair`, `::sign`, `::verify`, and all three members of
`Core\Crypto\KeyPair`. Each carries `about.md`, three examples with blessed `.out`, one attack, one
bench with a recorded figure, and a Rust test marked `covers:`.

The three left are `Core\Crypto\PublicKey`'s members, and **their proof paths spell the class with a
hyphen**: `docs/examples/core/Crypto-PublicKey/read/`, `benches/members/core/Crypto-PublicKey/kind.nvs`.
`python tools/dossier.py --id '<feature>'` prints all of them.

What the three take, from `nvs meta --json`: `read(bytes $encoded, Core\Crypto\KeyKind $kind,
Core\Crypto\KeyFormat $format)`, `write(Core\Crypto\KeyFormat $format): bytes`, `kind():
Core\Crypto\KeyKind`. A public key is a peer's, so its refusals are verdicts rather than bugs —
unlike `KeyPair::read`, whose every refusal is a `LogicError`.

**Key material for a `.nvs` proof is already written down**, so none of it has to be derived again:
`tests/conformance/core/crypto-key-pair-reads-every-kind-and-derives-the-half-webcrypto-exported.nvst`
holds all five kinds as hex, PKCS#8 beside SPKI, and
`crypto-key-pair-refuses-a-file-that-is-not-a-pkcs8-of-that-kind.nvst` holds the files that are
refused. The Ed25519 pair of that set writes the public half
`fcce9008a33565248561295a7ce1cccb040b6129bba8ff24744953bbc01261ac` in `Raw`.

Driving one of these members from Rust: the kind is a case index (`KeyKind::from_tag`), the frozen
set is `webcrypto::node("/jws/keys/ec-1")` with `/pkcs8`, `/spki` and `/pem` under it and
`/ecdh/vectors/1/a` for X25519, a member answering an instance is read back with `stored_key(&[answer],
0, &PUBLIC_KEY, "…")` plus `stored_octets` and then released under an `#[expect(unsafe_code)]`, and the
class a member threw is not on `nvs_runtime::call`'s error, so a refusal is asserted as the sentence
`ctx.take_pending()` answers.

## Next group

**Goal `Core\Crypto and 2 more`, stage: the public key class** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, `docs/examples/core/Crypto-PublicKey/`,
`tests/hostile/core/Crypto-PublicKey/`, `benches/members/core/Crypto-PublicKey/`. The three are one
class: the half that arrives, the half that is sent, and what it says it is. Each slice is
`rule:testing/feature-proofs`' full set for one member.

- [ ] **`Core\Crypto\PublicKey::read`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:1111`
- [ ] **`Core\Crypto\PublicKey::write`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:1130`
- [ ] **`Core\Crypto\PublicKey::kind`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:1139`

## Backlog

- `Core\Crypto\KeyPair::publicKey` measures 76 µs per call, because the half is worked out from the
  private key every time; `KEY_PAIR`'s own doc already prices the cache it declined.
- `Core\Secret::revealBytes` accepts an empty reason, which the attack on `KeyPair::write` step 4
  shows; nothing decided says it must not, so this is a question rather than a gap.
