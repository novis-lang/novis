# Handoff

## State

Goal `Core\Arr` (1/4). `product` and `average` are finished this session, beside the 53 members
that landed before them: `about.md`, three examples, one attack, one bench with a row in
`docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each. 55 of
`Core\Arr`'s 56 members are complete and `shapeAs` is the one owed. Nothing is blocked.

Both members reach `sum`'s `fold_numbers`, so each Rust test pins only what differs. `product`'s
identity is `int` `1`, and its overflow throws where `array_product` answers a `float`. `average`
answers `null` over the empty array and divides an exact total exactly: three `decimal` entries
totalling `1` over a count of `3` is `1/3` at the widest scale the quotient admits, where every
other total divides as a `float`.

`product` over a six-entry list is 199.4 ns/op at a 2.7 ns calibration unit, and `average` over six
entries is 208.8 ns/op at a 3.1 ns unit — 74.6 and 66.3 calibration units. Each is 4 statements, 0
calls, 3 allocations and 304 bytes per op, the same counts the other folds carry: the allocations
are the subject literal the round builds, since both answer a scalar and allocate nothing
themselves. Both declare `calls 0` and the measurement agrees.

Both attacks reach their last step and stop at the request's memory ceiling. `Core\Arr::average`
returns `?(float|decimal)`, and `?? 0.0` does not narrow it — a bench or example that stores the
mean writes `as float`, which is the union bullet already in the playbook.

## Next group

**One slice is one feature with all its feature proofs**, and `shapeAs` is the last one this group
owes — one file set: `crates/nvs-stdlib/src/arr.rs`, `docs/examples/core/Arr/`,
`tests/hostile/core/Arr/`, `benches/members/core/Arr/`. Write the `covers:` marker before the first
`--bless`: a `crates/` edit makes the release binary stale, and `--bless` then waits out a release
build of `nvs-cli`.

- [ ] **`Core\Arr::shapeAs`** — owes `about.md`, three examples, an attack, a bench, and a
      `covers:` marker over a Rust `#[test]`; `rule:testing/feature-proofs`. It reads an array as
      the type written at the call site, so every `.nvs` proof declares an inline shape or a
      `#[Json\Derive]` class, and a Rust `#[test]` installs a class table the way the playbook's
      `Ctx::class_desc` bullet describes — `crates/nvs-stdlib/src/arr.rs:6219`'s neighbours in the
      test module already build one. `crates/nvs-stdlib/src/arr.rs:6169`
- [ ] **Close the goal.** With `shapeAs` complete `python tools/dossier.py --gate --group
      'Core\Arr'` is green, and the goal's own end gates are what is left: `python tools/verify.py
      --doc`, `python tools/owners.py --closes core-arr-1-4` and `python tools/playbook.py --closes
      core-arr-1-4`, closing or re-ownering every gap they name. Then `.loop/status.txt` reads
      `DONE`. `docs/agent/loop-goal.md:4`

## Backlog

- `docs/perf/members.md` holds no row for any member this goal has measured; `python
  tools/dossier.py --perf-report` regenerates it from the ledger — `docs/perf/README.md`.
- `tools/data/dossier-policy.toml` has no `[skip]` entry this group needs: every `Core\Arr` member
  can carry all five proofs.
