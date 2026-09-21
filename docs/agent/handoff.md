# Handoff

## State

Goal `core-crypto-and-2-more` is met. All 15 features of `Core\Crypto`, `Core\Crypto\KeyPair` and
`Core\Crypto\PublicKey` carry their feature proofs: `python tools/dossier.py --gate` is green for
each of the three groups, and `--verify` runs their examples and attacks with nothing failed.

The three `Core\Crypto\PublicKey` members landed this session, each with `about.md`, three examples
with blessed `.out`, one attack, one bench with a recorded figure, and a Rust test marked `covers:`.
`Core\Crypto\PublicKey::kind` also needed its `covers:` marker on a `.nvst` case, since an instance
member is never credited by a plain call the way a static one is
(`docs/agent/conventions.md` § *Feature proofs*).

Figures appended to `docs/perf/members.ndjson`: `read` 341.0 ns/op and 3 allocations, `write`
95.7 ns/op and 3, `kind` 26.9 ns/op and none, which is what its bench declares.

## Next group

**Goal `core-csrf-and-3-more`, stage: the CSRF pair** — one file set: `crates/nvs-stdlib/src/csrf.rs`,
`docs/examples/core/Csrf/`, `tests/hostile/core/Csrf/`, `benches/members/core/Csrf/`. Each slice is
`rule:testing/feature-proofs`' whole set for one member, and the two members are one mechanism: the
token that is issued and the same token checked.

- [ ] **`Core\Csrf::issue`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/csrf.rs:113`
- [ ] **`Core\Csrf::verify`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/csrf.rs:126`
- [ ] **`Core\Csv::format`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/csv.rs:211`

## Backlog
- The generated dossier goals run in the order `docs/agent/goals/dossier/` numbers them; nothing
  else schedules them.
- A key's proofs draw their material from `tests/conformance/core/crypto-*.nvst` and
  `crates/nvs-stdlib/tests/vectors/webcrypto.json`, so nothing has to be derived again.
