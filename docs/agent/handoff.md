# Handoff

## State

Goal `core-bytes` — the feature proofs `rule:testing/feature-proofs` makes `Core\Bytes` owe. Three of
its thirteen members are complete: `at`, `compare` and `contains` each carry `about.md`, three
examples with blessed output, one attack, one recorded figure and one Rust test with a `covers:`
marker. Ten members are still owed; `python tools/dossier.py --owed --group 'Core\Bytes'` is the
list and the order.

Nothing is blocked, and no finding needed recording: the two allocations the first `compare` bench
measured are `nvs-ir`'s known gap 21, already written up there, and the naive search behind
`contains` and `indexOf` is the decision stated at `crates/nvs-stdlib/src/bytes.rs:751`. The attack
for `contains` meets that decision head on — five million bytes searched for a 5001-byte needle that
almost matches everywhere — and finishes in about 3.4 s on this host.

## Next group

One slice is one feature with all of its feature proofs. These three share the search half of
`crates/nvs-stdlib/src/bytes.rs` and the `find` helper under it, so the second and third cost a
fraction of the first. One file set: `crates/nvs-stdlib/src/bytes.rs` (the registry rows, the members
and the `tests` module at its foot), `docs/examples/core/Bytes/`, `tests/hostile/core/Bytes/`,
`benches/members/core/Bytes/`.

- [ ] **`Core\Bytes::indexOf`** — owes examples, hostile, perf, tests. Its third parameter is an
      options block, so a Rust-side call has to fill `from` the way a call site does rather than with
      `Value::null()` (the playbook bullet under *Writing a test case* on `Core\Arr::diff` is the
      shape). `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/bytes.rs:208`
- [ ] **`Core\Bytes::startsWith`** — owes examples, hostile, perf, tests.
      `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/bytes.rs:239`
- [ ] **`Core\Bytes::endsWith`** — owes examples, hostile, perf, tests.
      `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/bytes.rs:248`

## Backlog

- The seven builders and readers left in this class — `length`, `slice`, `fill`, `repeat`, `join`,
  `pack`, `unpack` — `python tools/dossier.py --owed --group 'Core\Bytes'`.
- `benches/members/lang/types/void-never-self-static.nvs` declares `allocations 2` and now measures
  1 on this host; it is not re-measured while its implementing file is unmoved, so nothing is red.
- There is no `bytes` literal, so every proof in this class builds its subject with
  `"…" as bytes`, `Core\Encoding::fromHex` or `Core\Bytes::fill` — `rule:types/bytes`.
