# Handoff

## State

Goal `Core\Crypto and 2 more` (15 features). Three are done and owe nothing: `Core\Crypto::agree`,
`::deriveKey` and `::expandKey` each carry `about.md`, three examples with blessed `.out`, one
attack, one bench with a recorded figure, and a Rust test marked `covers:`.

The two existing WebCrypto vector tests in `crates/nvs-stdlib/src/crypto.rs` were the Rust side of
all three; they pin exactly what those members do, so they were marked rather than duplicated.

A `Core\Crypto` member that answers `secret bytes` cannot be measured, printed or hex-encoded by a
Novis program, so every example and bench over one compares it against a value derived the same way.
That shapes each of the three benches: the round's input is chosen by whether the previous round's
key matched a key derived before the loop.

## Next group

**Goal `Core\Crypto and 2 more`, stage: the sealing half** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, `docs/examples/core/Crypto/`, `tests/hostile/core/Crypto/`,
`benches/members/core/Crypto/`. The three belong together: a key, and the two members that use one.
Each slice is `rule:testing/feature-proofs`' full set for one member.

- [ ] **`Core\Crypto::generateKey`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:656`
- [ ] **`Core\Crypto::seal`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:665`
- [ ] **`Core\Crypto::open`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:684`

## Backlog

- `Core\Crypto::generateKeyPair`, `::sign`, `::verify` — the goal's next three, same file set.
- `Core\Crypto\KeyPair::read`, `::write`, `::publicKey` — the goal's key-object half.
- `Core\Crypto\PublicKey::read`, `::write`, `::kind` — the same, for the half that is sent.
