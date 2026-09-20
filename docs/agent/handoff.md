# Handoff

## State

Goal `Core\Arr` (1/4). `Core\Arr::count`, `filter` and `map` are finished: `about.md`, three
examples, one attack, one bench with a row in `docs/perf/members.ndjson`, and a Rust `#[test]`
carrying its `covers:` marker each. 45 of `Core\Arr`'s features are still owed. Nothing is blocked.

Every `Core\Arr` figure was re-measured in one run at a 2.8 ns calibration unit, so the ten rows now
compare with each other: `count` 7.6 ns/op and no allocation, `map` 176 ns and 6, `filter` 389 and
16, where `all` and `any` spend 13 on the per-round closure alone. `withoutFirst` stands at 299 ns
and 11.

`filter`'s attack found one thing, recorded as `crates/nvs-stdlib/src/arr.rs`'s `# Known gaps` item
1 with owner M11: a callback that appends to the property the member is walking appends *under the
member's own cursor*, so the walk reaches those entries too and the request ends at its memory
limit. PHP's `array_filter` walks its own copy, and Novis's `foreach` holds a reference for the
loop's length so the body's write copies first. The fix is one retained reference for the length of
the walk in every member that calls back into Novis code, which prices a refcount pair onto every
one of those calls — the helper convention rather than one member, so it wants the user's word.

## Next group

**One slice is one feature with all its feature proofs**, taken in registry order so neighbours
share the declaration region they sit in — one file set: `crates/nvs-stdlib/src/arr.rs`,
`docs/examples/core/Arr/`, `tests/hostile/core/Arr/`, `benches/members/core/Arr/`. All three below
take a callback, so one understanding of what a callback is shown and of what happens to the keys
pays for the group. `filter`'s and `map`'s attacks are the shape to follow: a throwing callback, a
recursive one, a wide subject, then the step that ends at the memory ceiling.

- [ ] **`Core\Arr::mapKeys`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:148`
- [ ] **`Core\Arr::groupBy`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:167`
- [ ] **`Core\Arr::reduce`** — owes examples, hostile, perf, tests; `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/arr.rs:184`

## Backlog

- A closure that declares a return type and only throws is refused (`E0401: expected int, found
  never`), while the same closure with no declared type compiles. Whether `never` flows into a
  declared return is a language question no fragment under `docs/rules/` answers — the user's.
- 45 `Core\Arr` features still owe proofs, in registry order: `python tools/dossier.py --group
  'Core\Arr' --owed`.
- The two `Core\Arr::map` examples that predate `AGENTS.md` § *Text an end user reads* were brought
  up to it here; every other landed program is goal `plain-comments`'s to sweep.
