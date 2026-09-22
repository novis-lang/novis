# Handoff

## State

Milestone `dossier`, goal `core-encoding` is **met**. All eleven members carry their feature proofs
and the eleven figures are in `docs/perf/members.ndjson`: `python tools/dossier.py --verify --group
'Core\Encoding'` reports nothing owed, 33 examples and 11 attacks green; `python tools/verify.py
--doc` is green; `owners.py --closes core-encoding` and `playbook.py --closes core-encoding` name
nothing. Next in the chain is goal `core-env-and-4-more`.

The figures were taken in one release run at the end, because every session in this goal edited
`crates/nvs-stdlib/src/encoding.rs` and `rule:testing/member-perf-ledger` restales a figure the
moment that file's text moves.

`Core\Encoding::isValidText` measures 0.50 allocations per operation, so its bench declares `calls
0` and not `allocations 0` — the member performs and discards a decode
(`crates/nvs-stdlib/src/encoding.rs:1068`), which the allocator's per-thread totals see. Its Rust
proof replaced `validity_is_the_decode_question_without_the_throw`, which asked `decode_exact` the
same question one layer below the member.

## Next group

**Goal `core-env-and-4-more`, stage 1: the three `Core\Env` members — one file set:**
`crates/nvs-stdlib/src/env.rs` (the registry rows and the `mod tests` block), plus the proof trees
under `core/Env/<member>/`. `rule:testing/feature-proofs` says what each item owes and
`rule:testing/a-failing-proof-is-fixed-or-recorded` says what a finding does.

- [ ] **`Core\Env::get`** — `about.md`, three examples, one attack, one bench, a Rust `#[test]`
      carrying `// covers:`. It reads a value a caller does not control, so the attack is what a
      program does with a name that is absent, empty or enormous.
      `crates/nvs-stdlib/src/env.rs:112`
- [ ] **`Core\Env::all`** — the same five proofs. Its bench reads a whole map per operation, so it
      declares no `allocations 0`. `crates/nvs-stdlib/src/env.rs:121`
- [ ] **`Core\Env::mode`** — the same five proofs, over `Core\Env\Mode`'s cases.
      `crates/nvs-stdlib/src/env.rs:130`

## Backlog

- `Core\Encoding::isValidText` allocates one discarded decode per valid input, so a caller checking
  a large upload pays memory for a decode it throws away — recorded as a deliberate trade in
  `crates/nvs-stdlib/src/encoding.rs:1077`, not a gap.
- `Core\Fatal::onLimit`, `Core\Fatal::onUncaughtThrow` and the four `Core\Hash` members are the rest
  of goal `core-env-and-4-more`; `crates/nvs-stdlib/src/fatal.rs` and `hash.rs` are their file sets.
