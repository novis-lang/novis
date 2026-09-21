# Handoff

## State

Goal `core-config` is complete. Its acceptance check — `dossier: Core\Config` — reports nothing
owed, with 12 examples and 4 attacks green. All four members (`get`, `set`, `restore`, `all`) carry
an `about.md`, three blessed examples, one attack, one recorded bench figure and a Rust `#[test]`
with its `covers:` marker.

The three gates a goal meets only at its end are clean: `verify.py --doc` resolves every link, and
`owners.py --closes core-config` and `playbook.py --closes core-config` both report that this goal
owns nothing.

No proof found a bug, and the ledger's first figures raise one question, which is in `## Backlog`
rather than here: `restore` costs 21 allocations per call even when it drops nothing.

Two facts about writing a proof over this class. An example, an attack and a bench all run with the
repository's own `nvs.toml` in force, because the sweep runs them from the repository root, so
anything a proof prints has to be a value the program itself set — one that printed a value off the
file would freeze that file into a `.out`. And a request may set its own `limits.memory` to `1`,
which is accepted and then stops the program at its next allocation;
`tests/hostile/core/Config/set/01-a-request-raising-its-own-ceiling.nvs` ends on it.

## Next group

**Goal `core-crypto-and-2-more` — `Core\Crypto` and 2 more** — one file set:
`crates/nvs-stdlib/src/crypto.rs`, `docs/examples/core/Crypto/`, `tests/hostile/core/Crypto/`,
`benches/members/core/Crypto/`. All three are `rule:testing/feature-proofs`, and `Core\Config`'s
four landed members are the model for what each proof looks like.

- [ ] **`Core\Crypto::agree`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:740`
- [ ] **`Core\Crypto::deriveKey`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:697`
- [ ] **`Core\Crypto::expandKey`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/crypto.rs:718`

## Backlog

- `Core\Config::restore` refreshes the request's limits on every call, including one that dropped
  nothing — 699 ns and 21 allocations for a name never set. `nvs_config::Request::restore` answers
  nothing a caller could test on, so the fix starts there;
  `crates/nvs-stdlib/src/config.rs:217`.
- A `Core\Config` figure grows with the configuration the sweep runs under: `all` is 10.6 µs and 309
  allocations per call against the repository's 48-key `nvs.toml`, so it compares across commits
  only while that file keeps its size — `benches/members/README.md` § *When a figure is
  re-measured*.
