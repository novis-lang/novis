# Handoff

## State

Goal `lang-classes` is walking its item list: 15 features under `docs/reference/lang/50-classes.md`
owe feature proofs, and 11 are complete — `comparable`, `constants-and-class`, `declaring-a-class`,
`properties`, `methods-self-static-and-parent`, `inheritance`, `interfaces`, `property-hooks`,
`propertyobserver`, `stringable` and `parses`. Nothing is blocked.

Two attribution facts this group cost time to learn. A `Core`-side feature id and a chapter-side one
are **different features**: `tests/conformance/core/a-parse-answers-the-class-it-was-called-on-…`
already carried `// covers: Parses`, which credits the interface in the registry roster and not
`lang:classes/parses`, so it needed a second marker line. And `python tools/dossier.py --id '<id>'`
counts a language feature's cases from markers alone, so the corpus usually already holds the two —
reach for a marker, then write the one case the section's own claims leave unasked.

The three figures are all dominated by what the feature makes the runtime do besides dispatch. An
observed write plus an observed read is 60.5 ns (18.3 units, 3 statements, 2 calls, 0 allocations):
the observer is one ordinary virtual call per access and nothing else. Rendering an object into a
line of text is 90.3 ns (28.5 units, 1 call, 4 allocations, 146 bytes) — the call is cheap and the
string building around it is not. Building a value out of eight characters of text is 121.1 ns
(37.0 units, 2 calls, 5 allocations, 130 bytes).

## Next group

**The four sections that close the chapter — one file set:** `docs/reference/lang/50-classes.md`,
`docs/examples/lang/classes/<slug>/`, `tests/hostile/lang/classes/<slug>/`,
`benches/members/lang/classes/<slug>.nvs` and `tests/conformance/class/`. One slice is one feature
with all of its feature proofs (`rule:testing/one-slice-is-one-feature`), and `python
tools/dossier.py --id '<feature>'` prints the path of each. `python tools/dossier.py --verify --group
lang:classes` is the acceptance check and names what is left.

- [ ] **`lang:classes/objects-are-handles`** — owes examples, hostile, perf, tests. Two variables
      naming one object is what the section is written around, so the attack is a cycle the runtime
      has to free. `docs/reference/lang/50-classes.md:1123`
- [ ] **`lang:classes/nullable-objects-and`** — owes examples, hostile, perf, tests. `?->` short
      circuits, so the example nobody writes is the one where the call on the right has a side
      effect. `docs/reference/lang/50-classes.md:1163`
- [ ] **`lang:classes/object-the-top-of-every-class-type`** — owes examples, hostile, perf, tests.
      `object` is what a value is narrowed *from*, so the case worth writing asks what it still
      answers to. `docs/reference/lang/50-classes.md:1204`
- [ ] **`lang:classes/what-a-class-cannot-declare`** — owes examples, hostile, perf, tests. Every
      claim here is a refusal, so its cases are `--EXPECTF-ERROR--` and its examples have to show
      what to write instead. `docs/reference/lang/50-classes.md:1249`

## Backlog

- `Parses` as a registry feature (its own id, in the interfaces group) still owes its proofs —
  `python tools/dossier.py --id 'Parses'`.
- A user class implementing `Parses` has no `tryParse`: `Slug::tryParse(...)` is `E0309`, and
  `rule:expressions/try-parse` gives the twin only to a `parse` taking a plain `string`. The chapter
  section at `docs/reference/lang/50-classes.md:1052` says nothing either way.
- `docs/reference/lang/50-classes.md:940` says an observer is told of every access "in a subclass
  too"; `rule:classes/property-observer-pipeline`'s three boundaries — a hook's own slot, a `static`
  property, an observer that recurses — are in the rule and not in the chapter.
