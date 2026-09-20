# Handoff

## State

Goal `Core\Arr` (1/4). `fillKeys`, `range` and `fromKeysAndValues` are finished this session,
beside the 38 members that landed before them: `about.md`, three examples, one attack, one bench
with a row in `docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each.
41 of `Core\Arr`'s 56 members are complete and 15 are owed. Nothing is blocked.

**The attack written for `range` found a real gap, and this session fixed it.** The loop appended
without ever reading the request's balance, so `Core\Arr::range(1, 9000000000)` ran past ninety
seconds holding the host's memory, and `Core\Arr::range(1, 40000000)` reported its breach only
after a gigabyte had been built. The loop now asks `nvs_runtime::affordable` per entry, which is
what `nvs_runtime::sequence::drain` already does for `Core\Arr::from`, and
`tests/conformance/core/arr-range-over-a-span-too-wide-to-hold-is-stopped-by-the-memory-ceiling.nvst`
pins the stop. `rule:programs/memory-priority` is the rule it answers to.

None of the three owed a Rust test that was not already written, so that half of each slice was the
`covers:` marker and nothing else. At a 2.9 ns calibration unit `fillKeys` over six keys is
510.0 ns/op across 19 allocations, `range` over an eight-number span is 115.5 ns/op across 3, and
`fromKeysAndValues` over four pairs is 550.7 ns/op across 15. All three declare `calls 0` and the
measurement agrees. The other 38 rows in the ledger are re-measured because `impl_hash` moved with
this module's edit, which is `rule:testing/member-perf-ledger` working.

Each attack's last step reaches the request's memory ceiling, which is a `FATAL` no `try` catches,
so all three put that step last and declare `// hostile: ends-early`.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. **Write every
`covers:` marker of the group before the first `--bless`**: a `crates/` edit makes the release
binary stale, and `--bless` then waits out a release build of `nvs-cli`.

- [ ] **`Core\Arr::from`** — owes `about.md`, three examples, an attack, a bench, and a `covers:`
      marker over a Rust `#[test]`; `rule:testing/feature-proofs`. Its memory ceiling is already
      pinned by
      `tests/conformance/core/arr-from-over-a-sequence-with-no-end-is-stopped-by-the-memory-ceiling.nvst`,
      so the attack is the `{limit}` option and a cursor that never ends.
      `crates/nvs-stdlib/src/arr.rs:608`
- [ ] **`Core\Arr::overlay`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:620`
- [ ] **`Core\Arr::overlayDeep`** — owes the same five, and its examples are what separate it from
      `overlay`; `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/arr.rs:632`

## Backlog

- `Core\Arr::range`'s loop polls no deadline. A span inside the memory ceiling is bounded by that
  ceiling, so nothing runs unbounded, but `nvs_runtime::bounded_loop` is the seam if a CPU-limit
  case ever wants one — `rule:http-server/time-is-bounded-inside-a-helper`.
- A memory breach raised inside a native loop names the bytes held *after* the partial array was
  freed — 19,006 against a 256 MB ceiling — so the sentence reads as a refusal far below its own
  limit. `crates/nvs-runtime/src/ctx/limits.rs:338` is `memory_breach`.
- 15 `Core\Arr` members still owe their feature proofs, `shapeAs` among them with no test at all;
  `python tools/dossier.py --group 'Core\Arr'` is the list.
