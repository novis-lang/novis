# Handoff

## State

Goal `Core\Arr` (1/4). `min`, `max` and `sum` are finished this session, beside the 50 members that
landed before them: `about.md`, three examples, one attack, one bench with a row in
`docs/perf/members.ndjson`, and a Rust `#[test]` carrying its `covers:` marker each. 53 of
`Core\Arr`'s 56 members are complete and 3 are owed. Nothing is blocked.

`min` and `max` share one walk and so share their two Rust tests: the tie rule is pinned with the
float `1.0` ahead of the int `1`, which compare equal, so the answer's own tag says the **first**
extreme won. The divergence from PHP is pinned in the same pair — `[0, "a"]` throws where PHP
answers `0`, and `["1e2", "50"]` compares bytewise. `sum` carries its own two: the entry-by-entry
promotion with the empty array at `int` `0`, and the throw past `int`'s range where `array_sum`
becomes a `float`.

At a 3.3 ns calibration unit `min` over a six-entry list is 188.6 ns/op, `max` is 191.9 and `sum`
is 205.1, each across 3 allocations and 304 bytes — the subject literal the round builds, since all
three answer a scalar and allocate nothing themselves. All three declare `calls 0` and the
measurement agrees.

Every attack reaches its last step and stops at the request's memory ceiling. `Core\Arr::sum`
returns `int|float|decimal`, which is `mixed` in arithmetic — the first playbook bullet below is
what that costs a bench.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. **Write every
`covers:` marker of the group before the first `--bless`**: a `crates/` edit makes the release
binary stale, and `--bless` then waits out a release build of `nvs-cli`.

- [ ] **`Core\Arr::product`** — owes `about.md`, three examples, an attack, a bench, and a
      `covers:` marker over a Rust `#[test]`; `rule:testing/feature-proofs`. It is `sum`'s fold
      with the other operator, so the empty array is `int` `1` and the overflow throw is the same
      one; `fold_numbers` is shared and the proofs turn on what differs.
      `crates/nvs-stdlib/src/arr.rs:5839`
- [ ] **`Core\Arr::average`** — owes the same five; `rule:testing/feature-proofs`. It answers
      `?(float|decimal)`, so the empty array is `null` and a `decimal` subject stays exact while
      every other total is a `float`. `crates/nvs-stdlib/src/arr.rs:5858`
- [ ] **`Core\Arr::shapeAs`** — owes the same five; `rule:testing/feature-proofs`. It is the one
      member of this group whose arity is three more than its row's, because the call site's type
      argument arrives ahead of the declared parameters, and it reports through `Core\Json`'s
      `ParseError` rather than a throw of its own. `crates/nvs-stdlib/src/arr.rs:6169`

## Backlog

- `Core\Arr`'s benches all pay 3 allocations for the subject literal each round, so no member of
  this group can declare `allocations 0` — `benches/members/README.md` owns the declaration.
- Goal `plain-comments` sweeps the landed `.nvs` comments; `tests/hostile/core/Arr/sort/01-*.nvs`
  still opens with a 38-word line.
