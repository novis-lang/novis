# Handoff

## State

Goal `core-bytes` — the feature proofs `rule:testing/feature-proofs` makes `Core\Bytes` owe. Eleven
of its thirteen members are complete: the nine that measure and search, plus `repeat` and `join`,
each carrying `about.md`, three examples with blessed output, one attack, one recorded figure and
one Rust test with a `covers:` marker. `pack` and `unpack` are the two left, and `python
tools/dossier.py --owed --group 'Core\Bytes'` is that list.

Nothing is blocked, and no proof found a bug: both members answered what their reference cards say,
at both ends of their ranges. The figures are 175.1 ns/op with 3 allocations for `repeat` and 130.4
with 6.5 for `join`. `docs/perf/members.ndjson` carries an earlier `join` row of 168.6/8.0, measured
before its bench stopped indexing an array; the playbook bullet under *Running things* says why, and
`--id` reports the later row.

## Next group

One slice is one feature with all of its feature proofs. These two are each other's inverse and read
one format grammar between them — `packed` and `unpacked` share the `Field` shape and the same
letters — so the second is most of the first once the grammar is read. One file set:
`crates/nvs-stdlib/src/bytes.rs` (the registry rows, the two members, the two helpers above them and
the `tests` module at its foot), `docs/examples/core/Bytes/`, `tests/hostile/core/Bytes/`,
`benches/members/core/Bytes/`.

- [ ] **`Core\Bytes::pack`** — owes examples, hostile, perf, tests. It writes numbers into a buffer
      by a format string, which is the one member of this class whose second argument is a small
      language, so the examples are where a reader learns the letters.
      `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/bytes.rs:1461`, and the grammar it reads
      is `packed` at `crates/nvs-stdlib/src/bytes.rs:1343`.
- [ ] **`Core\Bytes::unpack`** — owes examples, hostile, perf, tests. It reads the same format back
      into an array, so its attack is a format that claims more bytes than the buffer holds.
      `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/bytes.rs:1697`, and its half of the
      grammar is `unpacked` at `crates/nvs-stdlib/src/bytes.rs:1615`.

## Backlog

- `docs/perf/members.md` is not rewritten by `--record-perf`; `python tools/dossier.py
  --perf-report` is what regenerates it from the ledger — `rule:testing/member-perf-ledger`.
- After `pack` and `unpack` this group owes nothing and the goal is met — `docs/agent/loop-goal.md`.
- `nvs-cli`'s `sd_notify_messages_are_ready_then_reloading_and_ready_then_stopping` failed once
  under load with an extra `STOPPING=1` first and passed alone and on the re-run; the notify
  socket it reads is shared with whatever runs beside it — `crates/nvs-cli/src/serve.rs:3913`.
