# Handoff

## State

Goal `Core\Arr` (1/4). `underlay`, `appendAll` and `diff` are finished this session, beside the 44
members that landed before them: `about.md`, three examples, one attack, one bench with a row in
`docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each. 47 of
`Core\Arr`'s 56 members are complete and 9 are owed. Nothing is blocked, and nothing this session
wrote found a bug: all three members answered what their examples claim, and all three attacks
reached their last step and stopped at the request's memory ceiling.

At a 3.0 ns calibration unit `underlay` over a two-key base and a six-key layer is 784.7 ns/op
across 13 allocations, `appendAll` over a six-value list and a two-value list is 248.0 ns/op across
7, and `diff` over two short lists is 844.2 ns/op across 26. All three declare `calls 0` and the
measurement agrees. `underlay` sits beside `overlay`'s 727.0 ns/op across 13, which is what says the
two share one walk.

The three Rust tests drive their members through `call` with the argument list the compiler would
have built: `underlay` and `appendAll` take a variadic tail as one `array` holding the layers, and
`diff` takes five arguments with `on` as `Core\SetOn::Values`' own number. `entries_of`, the helper
all three assert through, reads a string value and panics on any other tag, so a claim about an int
is made from the other side of the comparison.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. **Write every
`covers:` marker of the group before the first `--bless`**: a `crates/` edit makes the release
binary stale, and `--bless` then waits out a release build of `nvs-cli`.

- [ ] **`Core\Arr::intersect`** — owes `about.md`, three examples, an attack, a bench, and a
      `covers:` marker over a Rust `#[test]`; `rule:testing/feature-proofs`. It is `diff` with the
      other answer kept, through the same `set_member` walk, so its five arguments are built the
      same way and what the examples separate is which side is kept.
      `crates/nvs-stdlib/src/arr.rs:681`
- [ ] **`Core\Arr::countBy`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:694`
- [ ] **`Core\Arr::unique`** — owes the same five; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:703`

## Backlog

- `Core\Arr::min`, `max`, `sum`, `product`, `average` and `shapeAs` are the six the group owes after
  this one; `python tools/dossier.py --owed` is the live list.
