# Handoff

## State

Goal `Core\Crypto and 2 more` (15 features). Nine are done and owe nothing: the six sealing and
agreement members, plus `Core\Crypto::generateKeyPair`, `::sign` and `::verify`. Each carries
`about.md`, three examples with blessed `.out`, one attack, one bench with a recorded figure, and a
Rust test marked `covers:`. `python tools/dossier.py --owed --group 'Core\Crypto'` is now empty.

The six left are the two key classes, and **their proof paths spell the class with a hyphen**:
`docs/examples/core/Crypto-KeyPair/read/`, `benches/members/core/Crypto-PublicKey/write.nvs`.
`python tools/dossier.py --id '<feature>'` prints all four.

Driving one of these members from Rust: the kind is a case index (`0` P256, `1` X25519, `2` Ed25519,
`3` RsaPkcs1, `4` RsaPss — `KeyKind::from_tag` at `crates/nvs-stdlib/src/crypto.rs:1823`), a member
answering an instance is read back with `stored_key(&[answer], 0, &KEY_PAIR, "…")` and then released
through `Value::release` under an `#[expect(unsafe_code)]`, and the class a member threw is not on
`nvs_runtime::call`'s error, so a refusal is asserted as the sentence `ctx.take_pending()` answers.

## Next group

**Goal `Core\Crypto and 2 more`, stage: the key classes** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, `docs/examples/core/Crypto-KeyPair/`,
`tests/hostile/core/Crypto-KeyPair/`, `benches/members/core/Crypto-KeyPair/`. The three are one
class: the pair a program stores, and the two halves it answers. Each slice is
`rule:testing/feature-proofs`' full set for one member.

- [ ] **`Core\Crypto\KeyPair::read`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:1257`
- [ ] **`Core\Crypto\KeyPair::write`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:1274`
- [ ] **`Core\Crypto\KeyPair::publicKey`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:1283`

## Backlog

- `Core\Crypto\PublicKey`'s `read`, `write` and `kind` are the goal's last group —
  `python tools/dossier.py --owed --group 'Core\Crypto\PublicKey'`.
- `Core\Crypto::sign` costs about 72 µs per call for an Ed25519 message of 18 octets, which is what
  `docs/perf/members.ndjson` now holds; `sign` builds a `ring::rand::SystemRandom` on every call
  (`crates/nvs-stdlib/src/crypto.rs:2967`) whether the scheme draws randomness or not.
