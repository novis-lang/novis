# Handoff

## State

Goal `Core\Crypto and 2 more` (15 features). Six are done and owe nothing: `Core\Crypto::agree`,
`::deriveKey`, `::expandKey`, `::generateKey`, `::seal` and `::open` each carry `about.md`, three
examples with blessed `.out`, one attack, one bench with a recorded figure, and a Rust test marked
`covers:`. The nine left are the signing half and the two key classes.

A `secret bytes` cannot be printed or hex-encoded, and `Core\Secret::revealBytes($value, "reason")`
is the one route from it to a plain `bytes`. That is what an example uses to store a key or to seal
one under another key, and it is the shape `Core\Crypto::seal`'s reference card asks for.

Driving one of these members from Rust needs the cipher as its case index — `0` is
`XChaCha20Poly1305` and `1` is `Aes256Gcm`, read by `keyed` at `crates/nvs-stdlib/src/crypto.rs:3007`
— and the class a member threw is not on `nvs_runtime::call`'s error, so a refusal is asserted as
the sentence `ctx.take_pending()` answers.

## Next group

**Goal `Core\Crypto and 2 more`, stage: the signing half** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, `docs/examples/core/Crypto/`, `tests/hostile/core/Crypto/`,
`benches/members/core/Crypto/`. The three belong together: a key pair, and the two members that use
one. Each slice is `rule:testing/feature-proofs`' full set for one member.

- [ ] **`Core\Crypto::generateKeyPair`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:731`
- [ ] **`Core\Crypto::sign`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:756`
- [ ] **`Core\Crypto::verify`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:771`

## Backlog

- The six instance members of `Core\Crypto\KeyPair` and `Core\Crypto\PublicKey` are this goal's last
  group, behind the signing half — `docs/agent/loop-goal.md` § *The item list*.
- Sealing a 15-byte field costs 1077 ns/op and opening one 1053 ns/op, against 194 ns for
  `generateKey`: `keyed` builds the cipher, a key schedule, on every call. Measured and not
  investigated — `docs/perf/members.ndjson`.
