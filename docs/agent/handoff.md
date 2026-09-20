# Handoff

## State

Goal `core-bytes` — the feature proofs `rule:testing/feature-proofs` makes `Core\Bytes` owe. Nine of
its thirteen members are complete: `length`, `at`, `slice`, `compare`, `contains`, `indexOf`,
`startsWith`, `endsWith` and `fill` each carry `about.md`, three examples with blessed output, one
attack, one recorded figure and one Rust test with a `covers:` marker. Four are still owed;
`python tools/dossier.py --owed --group 'Core\Bytes'` is the list and the order.

Nothing is blocked, and no finding needed recording: each of the three members answered exactly what
its reference card says, at both ends of its range. The figures are 19.7 ns/op and no allocation for
`length`, 29.5 with one for `slice`, and 72.5 with three for `fill`. `python tools/dossier.py
--record-perf` appends to `docs/perf/members.ndjson` and does not touch `docs/perf/members.md`.

## Next group

One slice is one feature with all of its feature proofs. These two are the members that build a
buffer, and they share the tail of `crates/nvs-stdlib/src/bytes.rs` — `fill`'s neighbours there, and
the `affordable`/`reserved` pair all three ask before they allocate, so the attack this session
wrote against `fill` is most of the shape of theirs. One file set:
`crates/nvs-stdlib/src/bytes.rs` (the registry rows, the members and the `tests` module at its
foot), `docs/examples/core/Bytes/`, `tests/hostile/core/Bytes/`, `benches/members/core/Bytes/`.

- [ ] **`Core\Bytes::repeat`** — owes examples, hostile, perf, tests. It repeats a whole buffer
      where `Core\Bytes::fill` repeats one octet, which is the difference its examples exist to
      show. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/bytes.rs:1030`
- [ ] **`Core\Bytes::join`** — owes examples, hostile, perf, tests. Its separator defaults to the
      empty buffer (`Const::Bytes(b"")`), so the plain concatenation and the delimited one are the
      same member. `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/bytes.rs:1059`

## Backlog

- `Core\Bytes::pack` and `Core\Bytes::unpack` are the last two, and are a group of their own: both
  read the format grammar in `packed`/`unpacked` rather than the index helpers, at
  `crates/nvs-stdlib/src/bytes.rs:1461` and `:1697`.
- `docs/perf/members.md` is regenerated from the ledger by `python tools/dossier.py --perf-report`,
  which no session in this goal has run.
- An example that catches a `Core` throw reads `$error->message`; the exception tree declares
  properties, and `getMessage()` is `E0405` with the help line that says so.
