# Handoff

## State

Goal `lang-classes` is walking its item list: 15 features under `docs/reference/lang/50-classes.md`
owe feature proofs, and 2 are complete — `lang:classes/comparable` and
`lang:classes/constants-and-class`. Nothing is blocked.

The tests half of both was satisfied the way `rule:testing/proof-attribution` intends: a `covers:`
marker on each case that already pins the behaviour, plus one new case where a boundary was
genuinely unasked. `tests/conformance/class/` already holds five `Comparable` cases and the suite
holds twenty-eight class-constant ones, so reach for a marker before a new case here.

The perf proof of `constants-and-class` found an opportunity, not a bug: a runtime `$obj::class`
read allocates a fresh 38-byte string, where the folded `Class::class` form allocates nothing. The
bench declares `allocations 1` so that a change either way is visible; the `## Backlog` entry below
is the decision it waits on.

## Next group

**Declaring a class, its properties and its methods — three adjacent sections of one chapter.** One
file set: `docs/reference/lang/50-classes.md`, `docs/examples/lang/classes/<slug>/`,
`tests/hostile/lang/classes/<slug>/`, `benches/members/lang/classes/<slug>.nvs` and
`tests/conformance/class/`. One slice is one feature with all of its feature proofs
(`rule:testing/one-slice-is-one-feature`), and `python tools/dossier.py --id '<feature>'` prints the
path of each.

- [ ] **`lang:classes/declaring-a-class`** — owes examples, hostile, perf, tests.
      `rule:core-api/written-visibility` is what every member declaration answers to.
      `docs/reference/lang/50-classes.md:9`
- [ ] **`lang:classes/properties`** — owes examples, hostile, perf, tests.
      `rule:classes/definite-property-initialization` and `rule:classes/lateinit`.
      `docs/reference/lang/50-classes.md:105`
- [ ] **`lang:classes/methods-self-static-and-parent`** — owes examples, hostile, perf, tests.
      `rule:statements/static-is-a-member-modifier`.
      `docs/reference/lang/50-classes.md:301`

## Backlog

- A runtime `$obj::class` read allocates one 38-byte string per read. Interning a class name would
  make it free, and whether an immortal string is allowed at all is the question — no owner.
- Nine features of goal `lang-classes` remain after the group above: `inheritance`, `interfaces`,
  `nullable-objects-and`, `object-the-top-of-every-class-type`, `objects-are-handles`, `parses`,
  `property-hooks`, `propertyobserver`, `stringable` and `what-a-class-cannot-declare`.
  `docs/agent/loop-goal.md` § *The item list* carries the anchor of each.
- `verify.py`'s `nvs-fmt` step rewrites `tests/hostile/` in place, so a new attack file comes back
  formatted and has to be run again after the gate. `docs/examples/` is outside that step, where a
  landed example still reports a change under `nvs fmt --diff`.
