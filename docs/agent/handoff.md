# Handoff

## State

Goal `Core\Arr` (1/4). `column` and `sortByKey` are finished this session, beside the 34 members
that landed before them: `about.md`, three examples, one attack, one bench with a row in
`docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each. 36 of
`Core\Arr`'s 56 members are complete and 20 are owed. Nothing is blocked.

**The red floor check at session start was a deferral, not owed work.** `arr.rs` gap 4 — a member
that builds an array grows its storage as it appends — names M12, whose plan spoke only of the
optimising tier, so `python tools/owners.py --deferrals` refused it. M12's plan now states the
library's own allocation economy as its other half, which is where a `Core` member's deferred
allocation count belongs; that paragraph also covers `nvs-runtime` gap 10 and `arr.rs` gaps 2 and
3, which passed before on an accidental word match alone.

At a 3.1 ns calibration unit, `column` is 163.6 ns/op over 5 allocations for a three-row table and
`sortByKey` 293.5 ns/op over 9 for a three-entry array; both benches declare `calls 0` and the
measurement agrees.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. `sort` is the
partial one: it has two examples and an attack already, and owes a third example, a bench and a
Rust test. **Write every Rust test of the group before the first `--bless`**, because a `crates/`
edit makes the release binary stale and `--bless` then waits out the relink itself.

- [ ] **`Core\Arr::sort`** — owes `about.md`, a third example, perf and a Rust test with its
      `covers:` marker; `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/arr.rs:544`
- [ ] **`Core\Arr::fill`** — owes `about.md`, examples, hostile, perf and a Rust test with its
      `covers:` marker; `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/arr.rs:569`
- [ ] **`Core\Arr::fillKeys`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:578`
- [ ] **`Core\Arr::range`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:587`

## Backlog

- `Core\Arr::flip` costs 682.6 ns/op over 33 allocations for six entries, four times what its
  neighbours cost; why has not been checked. `docs/perf/members.ndjson`.
- 20 `Core\Arr` features still owe proofs, `sort` and then `fill` onward in registry order;
  `python tools/dossier.py --id '<feature>'` prints what each one owes.
