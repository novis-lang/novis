# Handoff

## State

Goal `core-bytes` — the feature proofs `rule:testing/feature-proofs` makes `Core\Bytes` owe. Six of
its thirteen members are complete: `at`, `compare`, `contains`, `indexOf`, `startsWith` and
`endsWith` each carry `about.md`, three examples with blessed output, one attack, one recorded
figure and one Rust test with a `covers:` marker. Seven are still owed; `python tools/dossier.py
--owed --group 'Core\Bytes'` is the list and the order.

Nothing is blocked, and no finding needed recording. The three search members measured 72.2, 31.5
and 29.4 ns/op, each with no allocation and no call: `indexOf` scans a 47-byte message the naive
way, which is the decision stated at `crates/nvs-stdlib/src/bytes.rs:751`, and the two predicates
are one byte comparison. The acceptance check the driver reported red before this session was
`target/release/nvs.exe` older than the tree, which `--bless` rebuilds itself — the playbook bullet
under *Running things* owns that line — so the next sweep reports what the group actually owes
instead.

## Next group

One slice is one feature with all of its feature proofs. These three share the front of
`crates/nvs-stdlib/src/bytes.rs` — the registry rows and the two index rules under them — and
`slice` reads its offset through the same `offset` helper `indexOf`'s `from` does, so the second and
third cost a fraction of the first. One file set: `crates/nvs-stdlib/src/bytes.rs` (the registry
rows, the members and the `tests` module at its foot), `docs/examples/core/Bytes/`,
`tests/hostile/core/Bytes/`, `benches/members/core/Bytes/`.

- [ ] **`Core\Bytes::length`** — owes examples, hostile, perf, tests. It counts octets where
      `Core\Str::length` counts what a person sees, which is the difference its examples exist to
      show. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/bytes.rs:176`
- [ ] **`Core\Bytes::slice`** — owes examples, hostile, perf, tests. Its length parameter is
      nullable, so a Rust-side call fills the third argument with the `Const::Null` the row declares
      rather than omitting it. `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/bytes.rs:194`
- [ ] **`Core\Bytes::fill`** — owes examples, hostile, perf, tests. A byte above 255 throws rather
      than being truncated to its low octet, which is the edge its attack and its Rust test both
      want. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/bytes.rs:256`

## Backlog

- `Core\Bytes::repeat`, `join`, `pack` and `unpack` are what is left after the group above;
  `python tools/dossier.py --owed --group 'Core\Bytes'` is their order.
- `pack` and `unpack` already have format-level cases in `bytes.rs`'s own `tests` module, so part of
  what they owe is a `covers:` marker rather than a new test — `rule:testing/proof-attribution`.
- No `[context]` field prints a proof tree's README. `benches/members/README.md` § *What a bench
  declares* is what says a `// bench: calls 0` is judged on the very first run and a missed
  declaration is a failing proof; `shapes` prints `conventions.md` sections only, so a session
  writing its first bench pays two calls for that rule.
