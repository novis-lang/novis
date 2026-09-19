# Handoff

## State

Goal `lang-classes` is walking its item list: 15 features under `docs/reference/lang/50-classes.md`
owe feature proofs, and 3 are complete — `lang:classes/comparable`,
`lang:classes/constants-and-class` and `lang:classes/declaring-a-class`. Nothing is blocked.

The perf proof of `declaring-a-class` found a real cost, and it is fixed in the same slice.
`nvs_runtime::release::release_value` started its worklist as `vec![first]`, so **every** container
whose last reference went away paid a `Vec` allocation before anything was dismantled: two
allocations per `new` for an object whose slots hold no refcounted value, which is most objects.
The first entry is now dismantled out of a local and the `Vec` starts empty, so a leaf never
allocates one. The bench went from 2 allocations per `new` to 1, and the ledger figure is recorded
there (2 calls, 1 allocation, 64 bytes per operation).

Tests were satisfied the way `rule:testing/proof-attribution` intends: `covers:` markers on five
cases that already pinned the constructor, promotion, the no-constructor class and the two
refusals, plus one new case for the `protected` boundary and one Rust test for the allocation
count. Reach for a marker before a new case here.

## Next group

**The properties section and the methods section — two adjacent sections of one chapter.** One file
set: `docs/reference/lang/50-classes.md`, `docs/examples/lang/classes/<slug>/`,
`tests/hostile/lang/classes/<slug>/`, `benches/members/lang/classes/<slug>.nvs` and
`tests/conformance/class/`. One slice is one feature with all of its feature proofs
(`rule:testing/one-slice-is-one-feature`), and `python tools/dossier.py --id '<feature>'` prints the
path of each.

- [ ] **`lang:classes/properties`** — owes examples, hostile, perf, tests.
      `rule:classes/definite-property-initialization` and `rule:classes/lateinit` are what the
      section is written around. `docs/reference/lang/50-classes.md:104`
- [ ] **`lang:classes/methods-self-static-and-parent`** — owes examples, hostile, perf, tests.
      `rule:statements/static-is-a-member-modifier` for `static`, and
      `rule:core-api/written-visibility` for every declaration in it.
      `docs/reference/lang/50-classes.md:300`

## Backlog

- The folded `Class::class` allocates nothing while a runtime `$obj::class` read allocates a fresh
  38-byte string; whether that read should fold or intern is open —
  `benches/members/lang/classes/constants-and-class.nvs` declares `allocations 1` so either answer
  is visible. Owner: `docs/reference/lang/50-classes.md:373`.
- Twelve features in this goal still owe everything: `inheritance`, `interfaces`,
  `nullable-objects-and`, `object-the-top-of-every-class-type`, `objects-are-handles`, `parses`,
  `property-hooks`, `propertyobserver`, `stringable`, `what-a-class-cannot-declare` and the two in
  the group above. `python tools/dossier.py --group lang:classes` is the list.
- `release_value`'s worklist still allocates once for an object that owns another container; a
  small-vector of one or two entries would cover the common tree and is unmeasured.
  Owner: `crates/nvs-runtime/src/release.rs:1`.
