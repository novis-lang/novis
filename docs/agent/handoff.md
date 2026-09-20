# Handoff

## State

Goal `core-bigint-2-2` — `Core\BigInt` (2/2), twelve features. Nine carry every feature proof now:
`parse`, `pow`, `powMod`, `shl`, `shr`, `sign`, and this session's `sub`, `sqrt` and `toString`.
Three are open — `toDecimal`, `toInt`, `toUint` — and all of them live in one file,
`crates/nvs-stdlib/src/bigint.rs`, beside the nine that are done.

`python tools/dossier.py --record-perf --group 'Core\BigInt'` measured the three new benches; the
other eighteen keep their figures, since no implementation moved. Nothing is blocked.

## Next group

**`Core\BigInt`'s three narrowing members** — one file set: `crates/nvs-stdlib/src/bigint.rs` for
the registry row, the member and the Rust test, plus `docs/examples/core/BigInt/<member>/`,
`tests/hostile/core/BigInt/<member>/` and `benches/members/core/BigInt/<member>.nvs`. One slice is
one feature with all of `rule:testing/feature-proofs`'s proofs; `python tools/dossier.py --id
'<feature>'` prints the path each belongs at. Each of the three already carries one `.nvst` case
and owes the Rust half, so a `covers:` marker is part of what the slice writes. Each refuses a
value its scalar cannot hold, so every attack has a refusal to name the way `sqrt`'s named the
negative receiver.

- [ ] **`Core\BigInt::toInt`** — owes about, examples, hostile, perf, a Rust test.
      `crates/nvs-stdlib/src/bigint.rs:313`
- [ ] **`Core\BigInt::toUint`** — owes about, examples, hostile, perf, a Rust test.
      `crates/nvs-stdlib/src/bigint.rs:322`
- [ ] **`Core\BigInt::toDecimal`** — owes about, examples, hostile, perf, a Rust test.
      `crates/nvs-stdlib/src/bigint.rs:331`

## Backlog

- The three members above are all this goal has left; this session deferred nothing else.
- A group's Rust tests share `crates/nvs-stdlib/src/bigint.rs`, so they land as one commit behind
  the per-member ones — `git log` for `shl`, `shr` and `sign` is the shape.
