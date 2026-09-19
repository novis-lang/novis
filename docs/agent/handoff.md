# Handoff

## State

Goal `lang-classes` is walking its item list: 15 features under `docs/reference/lang/50-classes.md`
owe feature proofs, and 5 are complete — `comparable`, `constants-and-class`, `declaring-a-class`,
`properties` and `methods-self-static-and-parent`. Nothing is blocked.

Both features this session took were given their tests by `covers:` markers on cases the corpus
already held: five for `properties`, one per section of that half of the chapter (definite
initialization, defaults, `lateinit`, `readonly`, `static`), and four for the methods half. The class
corpus is deep, so reach for a marker before writing a case here.

The two figures are worth holding together. A round of property reads and writes is 0.46 units, five
statements and no allocation. A round of three method calls — an instance method, the `parent::`
version it overrides, and a static one — is 5.76 units and 18.1 ns, so a call costs about 6 ns and
outweighs the body it runs.

## Next group

**Inheritance and interfaces — two adjacent sections of one chapter.** One file set:
`docs/reference/lang/50-classes.md`, `docs/examples/lang/classes/<slug>/`,
`tests/hostile/lang/classes/<slug>/`, `benches/members/lang/classes/<slug>.nvs` and
`tests/conformance/class/`. One slice is one feature with all of its feature proofs
(`rule:testing/one-slice-is-one-feature`), and `python tools/dossier.py --id '<feature>'` prints the
path of each.

- [ ] **`lang:classes/inheritance`** — owes examples, hostile, perf, tests. The section runs
      `extends`, `abstract`, `final`, `is` and `class<T>`; `rule:classes/constructor-compatibility`
      is what the `class<T>` half is written around, and the section's own text specifies the rest.
      `docs/reference/lang/50-classes.md:429`
- [ ] **`lang:classes/interfaces`** — owes examples, hostile, perf, tests. `rule:classes/no-traits`,
      `rule:classes/interface-default-methods` and `rule:classes/delegation-by-field` are the three
      it is written around. `docs/reference/lang/50-classes.md:657`

## Backlog

- An uncaught `RecursionError` prints every frame it unwound, which is about 950 KB of JSON at the
  default depth; nothing caps a trace's length and no rule decides one. The guard itself is right.
  Owner: `crates/nvs-runtime/src/ctx/safepoint.rs:635`.
- The folded `Class::class` allocates nothing while a runtime `$obj::class` read allocates a fresh
  38-byte string; whether that read should fold or intern is open —
  `benches/members/lang/classes/constants-and-class.nvs` declares `allocations 1` so either answer
  is visible. Owner: `docs/reference/lang/50-classes.md:373`.
- Ten features in this goal still owe everything: `inheritance`, `interfaces`,
  `nullable-objects-and`, `object-the-top-of-every-class-type`, `objects-are-handles`, `parses`,
  `property-hooks`, `propertyobserver`, `stringable` and `what-a-class-cannot-declare`.
  `python tools/dossier.py --group lang:classes` is the list.
- `release_value`'s worklist still allocates once for an object that owns another container; a
  small-vector of one or two entries would cover the common tree and is unmeasured.
  Owner: `crates/nvs-runtime/src/release.rs:1`.
