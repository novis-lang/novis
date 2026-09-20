# Handoff

## State

Goal `Core\Arr` (1/4). `from`, `overlay` and `overlayDeep` are finished this session, beside the 41
members that landed before them: `about.md`, three examples, one attack, one bench with a row in
`docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each. 44 of
`Core\Arr`'s 56 members are complete and 12 are owed. Nothing is blocked.

**The attack written for `overlayDeep` found a crash, and this session fixed it.** `overlay_into`
and `merged` recursed into each other once per level of nesting, so two configuration trees nested
20,000 deep overflowed this process's native stack and ended it with `0xC0000409` — the worker and
every request on it, not the one that asked. The walk now carries the nesting on the heap in an
explicit stack, which is what `nvs_core_arr_flatten_deep` in the same file already does and for the
same reason: the depth is the caller's data. `tests/conformance/core/arr-overlay-deep-combines-a-nesting-deeper-than-a-native-stack.nvst`
combines two towers of 100,000 maps and reads the bottom one, so a return to recursion fails there.
`rule:programs/memory-priority` is the rule it answers to — the bound is the request's memory
ceiling.

`Core\Arr::from` owed no Rust test that existed, so `from_materialises_an_array_as_a_list` took the
marker; `overlay` and `overlayDeep` had none at all, and the two written for them drive the members
through `call` with a variadic tail built by hand (one `array` holding the layers).

At a 2.9 ns calibration unit `from` over a six-element array is 156.1 ns/op across 5 allocations,
`overlay` over a six-key base and a two-key layer is 727.0 ns/op across 13, and `overlayDeep` over a
two-section tree is 760.9 ns/op across 21. All three declare `calls 0` and the measurement agrees.
The other 41 rows are re-measured because `impl_hash` moved with the walk's rewrite, which is
`rule:testing/member-perf-ledger` working.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. **Write every
`covers:` marker of the group before the first `--bless`**: a `crates/` edit makes the release
binary stale, and `--bless` then waits out a release build of `nvs-cli`.

- [ ] **`Core\Arr::underlay`** — owes `about.md`, three examples, an attack, a bench, and a
      `covers:` marker over a Rust `#[test]`; `rule:testing/feature-proofs`. It shares
      `overlay`'s walk and never recurses, so what the examples separate is the direction: the
      left-hand value wins and a new key lands at the end.
      `crates/nvs-stdlib/src/arr.rs:644`
- [ ] **`Core\Arr::appendAll`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:656`
- [ ] **`Core\Arr::diff`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:668`

## Backlog

- 12 `Core\Arr` members still owe their feature proofs; `python tools/dossier.py --group 'Core\Arr'
  --owed` is the list, in registry order.
- `Core\Arr::overlay` spends 13 allocations on a six-key base and a two-key layer, which is the copy
  plus a key per entry written; nothing says that is wrong, and no other combination member has been
  read against it — `docs/perf/members.ndjson`.
