---
looper: 1
title: "foreach binds a key and a value over a Core\\ObjectMap"
summary: "Reached when a compiler-owned keyed iteration interface exists, Core\\ObjectMap implements it, foreach ($map as var $key => var $value) binds the map's key and value types, [] stays array-only, and the feature has its proofs and pages."
context: [docs/rules/iteration/foreach-subjects.md, docs/rules/iteration/two-interfaces.md, docs/rules/types/var-inference.md]
stages:
  - id: 1
    title: the rules say what a keyed loop binds, and the checker binds a map's key and value
    status: open
    context: [crates/nvs-hir/src/interfaces.rs, crates/nvs-types/src/iter_lib.rs, crates/nvs-types/src/expr/iteration.rs, crates/nvs-types/tests/foreach.rs, docs/rules/iteration/no-magic-collection-interfaces.md, docs/rules/iteration/concrete-generic-implements.md, docs/rules/types/no-user-generics.md, data/rules/iteration.json]
  - id: 2
    title: a keyed loop runs over a map and over a user class that implements the keyed pair
    status: open
    context: [crates/nvs-ir/src/lower/control.rs, crates/nvs-stdlib/src/cursor.rs, crates/nvs-stdlib/src/objmap.rs, crates/nvs-runtime/src/sequence.rs, tests/conformance/core/object-map-iterates-its-keys.nvst, tests/conformance/iter/foreach-over-an-iterable-object.nvst]
  - id: 3
    title: the editor shows the key's type, and the feature has its proofs
    status: open
    context: [crates/nvs-lsp/src/hints.rs, tests/lsp/hints/an-inferred-var-declaration-carries-the-type-it-hid.lspt, docs/examples/lang/iteration/chapter.md, docs/examples/lang/iteration/core-collections-are-iterable/about.md, docs/examples/types/Iterable/about.md, tests/hostile/lang/iteration/core-collections-are-iterable/01-collections-changed-while-they-are-walked.nvs, benches/members/lang/iteration/core-collections-are-iterable.nvs, docs/agent/prose.md]
  - id: 4
    title: every page that says a map loop gives only keys says what it gives now
    status: open
    context: [website/src/content/docs/syntax/control-flow/foreach.mdx, website/src/content/docs/syntax/control-flow/iterators.mdx, website/src/content/docs/syntax/basics/variables.mdx, docs/reference/core/ObjectMap.md, website/src/content/docs/in-depth/what-changed.mdx, docs/agent/prose.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: keyed-checker
    stage: 1
    name: "a key binding over a map binds the map's key and value types, var takes them, a tainted key stays tainted, a wrong written key is refused, the value-only loop over a map still binds its keys, and a key over an iterable without keys is still refused"
    run: [cargo, test, --test, foreach, --, a_key_binding_over_an_object_map_binds_the_maps_key_and_value_types, a_var_key_over_an_object_map_takes_the_maps_key_type, a_var_key_over_a_tainted_key_type_is_tainted_string, a_plain_string_key_over_a_tainted_key_type_is_refused, a_written_key_type_that_is_not_the_key_type_is_refused, a_value_only_foreach_over_an_object_map_still_binds_its_keys, a_value_only_foreach_over_a_keyed_only_class_binds_its_values, a_key_binding_over_an_iterable_without_keys_is_still_refused, a_keyed_iterable_wins_over_a_keyed_iterator_when_a_class_reaches_both, a_class_implementing_both_pairs_binds_by_the_loops_form]
    timeout: 1800
    tests:
      - a_key_binding_over_an_object_map_binds_the_maps_key_and_value_types
      - a_var_key_over_an_object_map_takes_the_maps_key_type
      - a_var_key_over_a_tainted_key_type_is_tainted_string
      - a_plain_string_key_over_a_tainted_key_type_is_refused
      - a_written_key_type_that_is_not_the_key_type_is_refused
      - a_value_only_foreach_over_an_object_map_still_binds_its_keys
      - a_value_only_foreach_over_a_keyed_only_class_binds_its_values
      - a_key_binding_over_an_iterable_without_keys_is_still_refused
      - a_keyed_iterable_wins_over_a_keyed_iterator_when_a_class_reaches_both
      - a_class_implementing_both_pairs_binds_by_the_loops_form
  - id: keyed-loop
    stage: 2
    name: "a keyed foreach runs over a map and over a user class, sees the map as it was when it began, a map passes where a KeyedIterable is declared, and every refusal of the keyed form is pinned"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/iter/a-foreach-over-an-object-map-binds-each-key-with-its-value.nvst, --case, tests/conformance/iter/a-var-key-and-value-over-an-object-map-take-the-maps-types.nvst, --case, tests/conformance/iter/a-value-only-foreach-over-an-object-map-still-walks-its-keys.nvst, --case, tests/conformance/iter/a-keyed-foreach-sees-the-map-as-it-was-when-the-loop-began.nvst, --case, tests/conformance/iter/a-class-implements-keyed-iterable-at-concrete-types.nvst, --case, tests/conformance/iter/a-class-implements-keyed-iterator-and-is-driven-directly.nvst, --case, tests/conformance/iter/an-object-map-passes-where-a-keyed-iterable-is-declared.nvst, --case, tests/conformance/reject/a-map-key-that-is-tainted-binds-tainted-whether-written-or-var.nvst, --case, tests/conformance/reject/a-written-key-type-that-is-not-the-maps-key-type-does-not-compile.nvst, --case, tests/conformance/reject/a-key-over-an-iterable-without-keys-is-still-refused.nvst, --case, tests/conformance/reject/a-keyed-interface-needs-concrete-type-arguments.nvst, --case, tests/conformance/reject/a-keyed-iterator-without-key-does-not-compile.nvst, --case, tests/conformance/reject/an-object-map-has-no-subscript-to-read-or-write.nvst]
    timeout: 1800
    tests:
      - tests/conformance/iter/a-foreach-over-an-object-map-binds-each-key-with-its-value.nvst
      - tests/conformance/iter/a-var-key-and-value-over-an-object-map-take-the-maps-types.nvst
      - tests/conformance/iter/a-value-only-foreach-over-an-object-map-still-walks-its-keys.nvst
      - tests/conformance/iter/a-keyed-foreach-sees-the-map-as-it-was-when-the-loop-began.nvst
      - tests/conformance/iter/a-class-implements-keyed-iterable-at-concrete-types.nvst
      - tests/conformance/iter/a-class-implements-keyed-iterator-and-is-driven-directly.nvst
      - tests/conformance/iter/an-object-map-passes-where-a-keyed-iterable-is-declared.nvst
      - tests/conformance/reject/a-map-key-that-is-tainted-binds-tainted-whether-written-or-var.nvst
      - tests/conformance/reject/a-written-key-type-that-is-not-the-maps-key-type-does-not-compile.nvst
      - tests/conformance/reject/a-key-over-an-iterable-without-keys-is-still-refused.nvst
      - tests/conformance/reject/a-keyed-interface-needs-concrete-type-arguments.nvst
      - tests/conformance/reject/a-keyed-iterator-without-key-does-not-compile.nvst
      - tests/conformance/reject/an-object-map-has-no-subscript-to-read-or-write.nvst
  - id: keyed-runtime
    stage: 2
    name: "every Core class the checker calls a KeyedIterable answers iterateKeyed at run time, and the map's keyed cursor pairs each key with its value over the map's own store"
    run: [cargo, test, --lib, --, every_keyed_iterable_class_answers_the_keyed_protocol, a_keyed_cursor_pairs_each_key_with_its_value, a_keyed_cursor_shares_the_maps_store]
    timeout: 1800
    tests:
      - every_keyed_iterable_class_answers_the_keyed_protocol
      - a_keyed_cursor_pairs_each_key_with_its_value
      - a_keyed_cursor_shares_the_maps_store
  - id: editor
    stage: 3
    name: "the editor's inlay hints show the type var gave a map loop's key and value, and a key from a map completes its class's methods"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/hints/a-foreach-over-a-map-shows-the-key-and-value-types.lspt, --case, tests/lsp/completion/a-key-from-a-map-completes-its-class-methods.lspt]
    timeout: 1800
    tests:
      - tests/lsp/hints/a-foreach-over-a-map-shows-the-key-and-value-types.lspt
      - tests/lsp/completion/a-key-from-a-map-completes-its-class-methods.lspt
  - id: feature-proofs
    stage: 3
    name: "the keys-and-values-from-a-map feature owes no proof"
    run: [bun, nv, proofs, --id, "lang:iteration/keys-and-values-from-a-map"]
    timeout: 900
  - id: keyed-iterable-proofs
    stage: 3
    name: "the KeyedIterable interface owes no proof"
    run: [bun, nv, proofs, --id, KeyedIterable]
    timeout: 900
  - id: keyed-iterator-proofs
    stage: 3
    name: "the KeyedIterator interface owes no proof"
    run: [bun, nv, proofs, --id, KeyedIterator]
    timeout: 900
  - id: site-changes
    stage: 4
    name: "What changed lists the keyed loop as new"
    run: [bun, nv, site, --check, changes]
    timeout: 900
  - id: site-review
    stage: 5
    name: "every user text this goal changed carries its newcomer review stamp"
    run: [bun, nv, site, --check, review]
    timeout: 900
---

# foreach binds a key and a value over a Core\ObjectMap

## Why

A loop over a `Core\ObjectMap` gives only its keys today, so every program that wants the value
writes `foreach ($m as Tag $t) { $w = $m->get($t) ?? 0; }`: a second hash lookup per entry, and a
`?V` the program has to unwrap although the key is known to be there. The person asked for the loop
every other language has, with the types taken from the map:

```nvs
foreach ($objectsByKey as var $key => var $object) { ... }
foreach ($weights as Tag $tag => int $weight) { ... }
```

`rule:iteration/two-interfaces` gives a cursor no key half, and `rule:iteration/foreach-subjects`
refuses a key over anything but an array (`E0444`). This goal adds a second, keyed pair of
compiler-owned interfaces, makes `Core\ObjectMap<K, V>` implement it, and lets a `foreach` with a key
binding bind `K` and `V`. The loop walks the store `object-collections-fast` made shareable, so it
copies nothing.

It is the second of three goals that build revocable objects: `object-collections-fast` →
`keyed-iteration` → `revocable-objects`. A revocable object is removed from every map that holds it,
and walking a map's pairs is how a program sees what is left.

## Standing decisions

**Gate (on the whole goal).** `docs/goals/object-collections-fast.md` does not exist. That goal makes
the map store a key directly and gives a `foreach` a cursor that shares the store instead of copying it,
and the keyed cursor here walks that store. It also lands the `website-follows-changes` rules this goal's
stage 4 follows. The anchors below in `crates/nvs-stdlib/src/objmap.rs`, `crates/nvs-stdlib/src/cursor.rs`
and `crates/nvs-stdlib/src/instance.rs` were taken before `object-collections-fast` rewrote those files,
so stage 2 finds each site by symbol (`bun nv peek --locate nvs_core_object_map_iterate`).

**The interfaces (the authoring session's call).** Two reserved names the compiler owns, beside
`Iterable` and `Iterator` in `nvs_hir::interfaces::RESERVED`:

```
interface KeyedIterator<K, V> {
    public function advance(): bool;   // move to the next entry; false once exhausted
    public function key(): K;          // the key of the entry advance() just moved to
    public function current(): V;      // the value of that entry
}

interface KeyedIterable<K, V> {
    public function iterateKeyed(): KeyedIterator<K, V>;
}
```

- The names keep the base pair's nouns, so a reader who knows `Iterable`/`Iterator` reads them at once.
- **`iterateKeyed`, not `iterate`.** `Core\ObjectMap` implements both `Iterable<K>` and
  `KeyedIterable<K, V>` (the next decision), and Novis has no overloading, so one class cannot have
  `iterate(): Iterator<K>` and `iterate(): KeyedIterator<K, V>`. The cursor pair shares `advance` and
  `current` with `Iterator<T>`, so one cursor class may implement both when `T` is `V`; with another `T`
  its `current()` cannot satisfy both, and that is the ordinary `E0449`/override mismatch, not a new code.
- `key()` and `current()` follow `rule:iteration/cursor-out-of-range`: called outside the protocol they
  throw the same `LogicError`.
- A user class implements either at concrete types only (`implements KeyedIterable<string, int>`), under
  `rule:iteration/concrete-generic-implements`; a type variable of its own stays refused
  (`rule:types/no-user-generics`).
- Declined, each with its reason in the new rule: **`Iterable<K, V>` with a second type argument** (a
  name's arity is one fact in `RESERVED`, and the map would have to be `Iterable<K>` and `Iterable<K, V>`
  at once); **a `key()` member on `Iterator<T>`** (every cursor and generator would owe a key it does
  not have, which is the counter `rule:iteration/foreach-subjects` refuses to invent); **`entries()` as
  the method name** (a user class is likelier to have its own `entries`).

**Which interface a loop drives (the authoring session's call).** The binding form decides, and the old
form keeps its meaning, so no existing program changes:

- **A loop with a key binding** drives the keyed pair. `KeyedIterable` wins over `KeyedIterator` when a
  class reaches both, for `resolve_iteration_element`'s reason (`crates/nvs-types/src/signatures.rs:2496`):
  a fresh cursor is the safer of the two. A subject that reaches only `Iterable`/`Iterator` is still
  `E0444`. An array keeps its own key rules.
- **A loop with a value binding alone** drives `Iterable`/`Iterator` when the subject reaches either, so
  a map still gives its keys. A subject that reaches only the keyed pair is walked through it and binds
  `V`, as a value-only loop over an array binds the element.
- `ForeachDrive` (`crates/nvs-types/src/expr_table.rs:1417`) gains `KeyedIterable` and `KeyedCursor`.
  Safe fallback if the IR forces one variant: a `keyed: bool` on the two cursor drives, with the same
  meaning.

**What the key binding gets.** `var $k` takes the subject's key type: `string` over an array (unchanged),
`K` over the keyed pair (`Tag` over `ObjectMap<Tag, int>`), and `K` keeps its qualifier, so a `tainted
string` key binds `tainted string`. A written key type over the keyed pair is checked as a value binding
is: assignable from `K`, so a wider type such as `object` is accepted. A plain `string` over a `tainted`
key is `E0851`, the refusal `report_untainted_outside_key` gives today
(`crates/nvs-types/src/expr/iteration.rs:468`). Any other mismatch is `E0401`. A `foreach` binding with
neither a type nor `var` stays the parse error it is.

**`[]` stays array-only (the person accepted it).** No subscript interface and no `Countable`.
`rule:iteration/no-magic-collection-interfaces` keeps its decision and its Declined list gains two
bullets: *a subscript interface*, because the same `[]` would write a copy on an array and shared state
on an object, and a nested write such as `$obj['x']['y'] = 1` writes into a temporary that is lost (PHP's
"Indirect modification of overloaded element"); and *a read-only subscript interface*, because it swaps
one surprise for another: `$x[$k] = $v` would not compile on something that reads like an array.

**What the loop sees.** The keyed cursor walks the store `object-collections-fast` shares with the loop,
with no copy. The loop sees the map as it was when it began, as the value-only loop does: an entry the
body adds is not visited, one it removes is still visited with the value it had, and a `set` on an entry
not yet reached does not change the value the loop gives. Whatever `object-collections-fast` records as
the cost of a write while a loop shares the store is the cost here too, and the rule cites it.

**Out of this goal, each staying as it is:** a keyed `yield` stays refused (`rule:iteration/one-way-only`),
so a generator is still an `Iterator<T>`; `inout` over the keyed pair is `E0490`, the same refusal as over
any cursor, and the program writes with `$map->set($key, ...)`; `Core\Arr::from($map)` still gives the
keys, since its parameter is `array|Iterable|Iterator`; `Core\ObjectSet`, `Core\Heap` and
`Core\Task\Channel` get no keyed pair, since none has a value apart from its element.

**Where each fact lives (the authoring session's call).** A new fragment,
`docs/rules/iteration/keyed-interfaces.md`, is the keyed pair's one home: the members, the drive choice
above, the method name and the Declined list. `rule:iteration/two-interfaces` keeps its id and becomes
the rule of the value pair: its first sentence says it covers iteration over values and names
the new rule `iteration/keyed-interfaces` for keys. Renaming it would rewrite 107 citations in 67 files, every
one of which means the value pair and stays true. `rule:iteration/foreach-subjects` lists five
subjects and takes the binding-form choice, `rule:types/var-inference` takes the key sentence,
`rule:security/taint-propagation` takes one sentence on a `K` that carries `tainted`, and
`rule:types/no-user-generics` lists the two new names.

**The feature names (fixed by the roster).** The language feature is a new section of
`docs/examples/lang/iteration/chapter.md`, `# Keys and values from a map`, after `# \`Core\` collections are \`Iterable\``,
so its roster id is `lang:iteration/keys-and-values-from-a-map`. If the roster slugs the heading another
way, the session rewrites the `feature-proofs` check's id to the landed one in the same slice. Each
`RESERVED` entry is an interface feature of its own (`bun nv proofs --id Iterable` shows the shape), so
`KeyedIterable` and `KeyedIterator` each owe what `bun nv proofs --id` prints for them. The
`# The two interfaces` section keeps its heading, so its feature id and its proofs do not move.

**The editor (the authoring session's call).** `crates/nvs-lsp/src/hints.rs:206` already hints every
`var` binding of a `foreach` header, the key included, from the type the checker records under the `var`
keyword, so the keyed form needs no code there, only the `.lspt` cases that prove it. Hover on a plain
variable stays silent, by `crates/nvs-lsp/src/hover.rs:62`'s design; that is a change to what the type
phase keeps and not this goal's.

**The text a user reads.** Every page, card and diagnostic this goal writes follows AGENTS.md
§ *Text an end user reads*. The prescribed wording:

- `E0443`'s card (`crates/nvs-diagnostics/src/code/types.rs:367`): "`foreach` can loop over an array,
  an `Iterable`, an `Iterator`, a `KeyedIterable` or a `KeyedIterator`, and this value is none of them.
  Loop over an array, or make the class implement one of these interfaces. Convert a `mixed` value
  first: write `$value as array<mixed>`." Its help in `report_not_iterable`
  (`crates/nvs-types/src/expr/iteration.rs:210`) names the same five.
- `E0444`'s card (`:376`): "A `foreach` over an `Iterable` or an `Iterator` has no keys, so `$k =>` is
  not allowed. Write `foreach ($items as string $v)`. To get keys, loop over an array or a
  `KeyedIterable`." Its help (`crates/nvs-types/src/expr/iteration.rs:433`): "an iterator has values
  but no keys. Remove the `$k =>`, or loop over an `array<T>` or a `KeyedIterable<K, V>`". The message
  line stays as it is.
- The new section's `about.md` opens: "A `foreach` loop over a `Core\ObjectMap` can give you each key
  with its value. Write `foreach ($map as var $key => var $value)`. `$key` gets the map's key type, and
  `$value` gets its value type."
- The `What changed` entry: "`foreach` gives the key and the value of a `Core\ObjectMap`: write
  `foreach ($map as var $key => var $value)`. A class of your own can do the same by implementing
  `KeyedIterable<K, V>`."

**What it costs.** One more pair of interfaces to learn, beside the two a reader already knows. Memory:
none beyond today's loop. The loop holds one cursor object, as the value-only loop does, and walks the
shared store. Speed: the loop grows as an array loop grows, linearly and with no allocation per entry.
Per entry it pays the dispatched calls a cursor loop pays (`advance`, `key`, `current`), where an array
loop pays none, and it saves the `get` lookup a program writes today. The bench records the keyed loop
beside an array loop over the same entries. The rule's *What it spends* line states the measured
figures, never an estimate.

**Lanes.** Stages 1 and 2 run in order: stage 2 lowers the drive stage 1 records. The editor items of
stage 3 touch only `tests/lsp/` and may run as a side goal any time after stage 1. The rest of stage 3
needs stage 2, since every example and attack runs. Stage 4 needs stage 3, because its pages link the new
section and quote its examples. Stage 5 is last.

**Limits: what stops this goal.** One attempt per item, and at most two sessions per stage. An item
still red after its attempt goes into the handoff with its cause, and the stage moves on to its other
items. A stage still open after two sessions stops the loop with `BLOCKED` naming the item. The counts
decide a perf question and the clock only reports. A check is never loosened, a test never renamed to
pass, and a frozen expected output never edited to fit.

## Stages

### 1. the rules say what a keyed loop binds, and the checker binds a map's key and value

**Gate:** `docs/goals/object-collections-fast.md` does not exist.

**Does:** writes the rules, adds the keyed pair to the reserved roster with its seeded members, and
teaches the checker to classify a keyed subject and bind `K` and `V`. Nothing runs yet: a keyed loop
type-checks and the IR still panics on it, which stage 2 removes.

**Split:** an `Explore` subagent lists every place that matches a reserved interface by the name
`Iterable` or `Iterator` and would now meet a keyed name too (`is_reserved_global_interface`,
`nvs_hir::interfaces::ITERABLE`, `ITERATOR`, across `crates/`), as `path:line` and one line each. The
session writes the code.

- [ ] `docs/rules/iteration/keyed-interfaces.md:1`: the new rule from § *Standing decisions* (the
  members, the drive choice, `iterateKeyed`, the key binding, what the loop sees, what it spends and the
  Declined list). `data/rules/iteration/keyed-interfaces.json:1` with its `seeAlso` and `guardedBy`,
  and its place in `data/rules/iteration.json:1`.
- [ ] The rule texts that change: `docs/rules/iteration/two-interfaces.md:1` (iteration over values,
  pointing at the keyed pair), `docs/rules/iteration/foreach-subjects.md:1` (five subjects and the
  binding-form choice), `docs/rules/types/var-inference.md:1` (the key sentence),
  `docs/rules/iteration/no-magic-collection-interfaces.md:16` (the two Declined bullets),
  `docs/rules/security/taint-propagation.md:40`, `docs/rules/types/no-user-generics.md:7`, and the
  titles in `data/rules/iteration/two-interfaces.json:2` and `data/rules/iteration/foreach-subjects.json:2`.
  `bun nv rules --render`, then `bun nv rules --check` is green.
- [ ] `crates/nvs-hir/src/interfaces.rs:45`: `KeyedIterable` and `KeyedIterator` on `RESERVED`, each
  with `["K", "V"]`, their constants beside `ITERATOR` (`:104`), and the module test of the type
  parameters. `crates/nvs-types/src/lower.rs:974`: the `E0441` help names the two.
- [ ] `crates/nvs-types/src/iter_lib.rs:208`: the keyed pair seeded beside `Iterator`/`Iterable`, with a
  two-parameter twin of `elem_var` (`:240`), the module doc's roster count, and a test in `mod tests`
  (`:299`) beside `the_cursor_declares_its_two_members`. `crates/nvs-runtime/src/sequence.rs:82`:
  `ITERATE_KEYED` and `KEY` beside `ITERATE`, `ADVANCE` and `CURRENT`.
- [ ] The classification, in one group of files:
  - [ ] `crates/nvs-types/src/signatures.rs:2496`: `resolve_keyed_iteration` beside
    `resolve_iteration_element`, returning the interface and `(K, V)`.
  - [ ] `crates/nvs-types/src/expr/iteration.rs:103`: `ForeachSource::Keyed { key, value, via_iterable }`,
    and `classify_foreach_source` (`:158`) takes whether the loop binds a key and applies the drive
    choice; the reserved-name arm at `:175` tells the keyed names from the value names.
    `crates/nvs-types/src/expr_table.rs:1417`: the two new `ForeachDrive` variants.
  - [ ] `crates/nvs-types/src/expr/iteration.rs:365` and `:408`: the key type comes from the source
    (`K` for a keyed source, the array rule otherwise), and a written key over a keyed source is checked
    as § *Standing decisions* says. `check_foreach_inout` (`:297`) gives a keyed source `E0490`.
  - [ ] `crates/nvs-types/src/locals.rs:1655`: the `foreach` arm passes `key.is_some()` to the
    classification and takes the key type from the source.
  - [ ] `crates/nvs-types/src/expr/quals.rs:229`: `carries_tainted` reads both arguments of the keyed
    pair, as it reads `Iterable`'s one.
- [ ] `crates/nvs-diagnostics/src/code/types.rs:367` and `:376`: the `E0443` and `E0444` cards, and their
  helps at `crates/nvs-types/src/expr/iteration.rs:210` and `:433`, in the wording of § *Standing
  decisions*. `docs/examples/lang/iteration/the-two-interfaces/04-an-iterator-has-no-key.out:6` and
  `tests/conformance/iter/a-generator-has-no-key-to-bind.nvst:1` follow the new help (a changed help
  line, never a changed verdict).
- [ ] `crates/nvs-types/tests/foreach.rs:1`: the ten tests the `keyed-checker` check names, one with
  `// covers: lang:iteration/keys-and-values-from-a-map`. The map tests declare
  `Core\ObjectMap<Tag, int>` with a small `Tag` class; the tainted ones use
  `Core\ObjectMap<tainted string, int>`. Run the check once.

### 2. a keyed loop runs over a map and over a user class that implements the keyed pair

**Does:** lowers the keyed drive, gives `Core\ObjectMap` its keyed cursor over the shared store, and
seeds the map's `KeyedIterable<K, V>` edge for the checker. Then the conformance cases pin the whole
feature from Novis.

**Split:** once the loop runs, a writing subagent writes the thirteen `.nvst` cases the `keyed-loop`
check names, copying the shape of `tests/conformance/iter/foreach-over-an-iterable-object.nvst` and
`tests/conformance/core/object-map-iterates-its-keys.nvst`. It is told § *Standing decisions*
*Which interface a loop drives*, *What the key binding gets* and *What the loop sees*. Each case carries
a `// covers:` line: `lang:iteration/keys-and-values-from-a-map` for the map cases, `KeyedIterable` and
`KeyedIterator` for the two user-class cases. The session runs the check once.

- [ ] `crates/nvs-ir/src/lower/control.rs:903`: the non-array branch keeps the `inout` assert, drops the
  key assert for a keyed drive, and passes the key binding on. `lower_foreach_cursor` (`:1297`) takes
  the key binding and whether the drive is keyed: it calls `iterateKeyed` in place of `iterate`, and
  after each `advance` calls `key` (when a key is bound) and then `current`, each through
  `emit_iface_call` with the same retain discipline, and widens the key as it widens the value. The
  doc comment states the order and that a keyed loop costs one more call per entry.
- [ ] `crates/nvs-stdlib/src/registry.rs:4216`: a `KEYED_ITERABLES` roster beside `ITERABLES`, one row
  `(r"Core\ObjectMap", Var("K"), Var("V"))`, and its lookup beside `iterable_element` (`:4299`).
  `crates/nvs-types/src/core_lib.rs:156` lowers both types and `:304` seeds the `KeyedIterable` edge.
  The test at `:2780` asserts the map reaches both edges.
- [ ] `crates/nvs-stdlib/src/cursor.rs:65`: a keyed cursor class beside `Core\Cursor`, internal as that
  one is, which walks the map's store with no copy and answers `advance`, `key` and `current`. Its module
  doc says what it spends. `crates/nvs-stdlib/src/objmap.rs:523`: `nvs_core_object_map_iterate_keyed`
  beside `nvs_core_object_map_iterate`, its symbol beside `ITERATE_SYMBOL` (`:52`) and its row in
  `address` (`:300`). The module doc's *A `foreach` over a map yields its keys* paragraph says which loop
  gives what.
- [ ] `crates/nvs-stdlib/src/instance.rs:143` and `:790`: the keyed cursor joins `INTERNAL_CLASSES`, the
  map's row gains `iterateKeyed`, and the cursor's row (`:1006`) has a keyed twin.
  `every_keyed_iterable_class_answers_the_keyed_protocol` beside
  `every_iterable_class_answers_the_iteration_protocol` (`:2159`).
  `a_keyed_cursor_pairs_each_key_with_its_value` and `a_keyed_cursor_shares_the_maps_store` in
  `crates/nvs-stdlib/src/cursor.rs`'s tests. The `keyed-runtime` check run once.
- [ ] Tried by hand before the cases: a keyed loop over a map, a value-only loop over a map, a user
  `KeyedIterable` and a user `KeyedIterator`, each printing what it walked.
- [ ] The thirteen `.nvst` cases of the `keyed-loop` check, and the check run once. The existing
  `tests/conformance/core/object-map-iterates-its-keys.nvst:1` stays green unchanged.

### 3. the editor shows the key's type, and the feature has its proofs

**Does:** proves the editor half, and writes every proof `rule:testing/feature-proofs` lists for
`lang:iteration/keys-and-values-from-a-map`, `KeyedIterable` and `KeyedIterator`. Every file here is text
an end user reads (AGENTS.md § *Text an end user reads*), and every `.nvs` comment passes
`bun nv proofs --comments`.

**Split:** three writing subagents, whose files are disjoint, run side by side once the session has
written the section's `about.md` and `details.md`: one writes the three examples and their `.out` files,
one the attack, one the bench. A fourth writes the two interface features' `about.md`, example and
`.out`. Each is told the section's path and § *Standing decisions*. The session writes the `.lspt`
cases, the section and the chapter line, and runs the four stage-3 checks once.

- [ ] `tests/lsp/hints/a-foreach-over-a-map-shows-the-key-and-value-types.lspt:1`: a `var` key and value
  over `Core\ObjectMap<Tag, int>` hint `: Tag` and `: int`, in the shape of
  `tests/lsp/hints/an-inferred-var-declaration-carries-the-type-it-hid.lspt`. Expected to need no code in
  `crates/nvs-lsp/src/hints.rs:206`; if the key hint is missing, the fix goes there.
- [ ] `tests/lsp/completion/a-key-from-a-map-completes-its-class-methods.lspt:1`: `$key->` inside the
  loop offers `Tag`'s methods. The `editor` check run once.
- [ ] `docs/examples/lang/iteration/chapter.md:12`: the heading `# Keys and values from a map`, and the
  front matter's `keywords` gain `KeyedIterable`, `KeyedIterator`, `iterateKeyed`, `key`, `key and value`.
  `docs/examples/lang/iteration/keys-and-values-from-a-map/about.md:1` opens with the wording of
  § *Standing decisions*, then says what the loop sees when the body changes the map, and that a class
  of your own implements `KeyedIterable<K, V>`. `details.md:1` lists the codes: `E0444` over an
  `Iterable`, `E0401` for a wrong key type, `E0851` for a plain `string` over a `tainted` key, `E0490`
  for `inout`. `bun nv render` writes the section into `docs/reference/lang/60-iteration.md`, which is
  the feature's help (`nvs agent find KeyedIterable` reaches it).
- [ ] Three examples in `docs/examples/lang/iteration/keys-and-values-from-a-map/`, each with its
  `.out`: `01-each-key-with-its-value` (written types over `ObjectMap<Tag, int>`),
  `02-var-takes-the-types-from-the-map` (`var` for both, and a use of each type),
  `03-a-class-of-your-own-with-keys-and-values` (a user class implementing `KeyedIterable<string, int>`
  with its own `KeyedIterator`).
- [ ] `tests/hostile/lang/iteration/keys-and-values-from-a-map/01-a-map-changed-while-its-pairs-are-walked.nvs:1`,
  in the shape of `tests/hostile/lang/iteration/core-collections-are-iterable/01-collections-changed-while-they-are-walked.nvs`.
  Its top comment:

  ```nvs
  // Attack: a program changes a `Core\ObjectMap` while a loop walks its keys and values.
  // Each loop should end after the entries it started with, and the runtime should stay standing.
  ```

  The steps:
  remove every entry while walking; remove and add back each entry with a new value while walking;
  nest keyed loops over one map and write at each depth; grow a map inside a keyed loop until the
  memory limit stops the program.
- [ ] `benches/members/lang/iteration/keys-and-values-from-a-map.nvs:1`: a keyed loop over a map of
  100,000 entries beside an array loop over the same entries, summing the values in each.
  `// bench: complexity linear`. The ramp's verdict and the per-op allocation count go into the rule's
  *What it spends* line.
- [ ] `docs/examples/types/KeyedIterable/about.md:1` and `docs/examples/types/KeyedIterator/about.md:1`,
  each in the shape of `docs/examples/types/Iterable/about.md`, with one example and its `.out` each.
  The two user-class cases of stage 2 carry their `// covers:` lines.
- [ ] The `feature-proofs`, `keyed-iterable-proofs` and `keyed-iterator-proofs` checks run once, and
  every proof they name written.

### 4. every page that says a map loop gives only keys says what it gives now

**Does:** every page, card and example that says a map loop gives only its keys, or that only an array
has keys, says what is true now, and the website follows the `website-follows-changes` rules. Every
line here is text an end user reads.

**Split:** a writing subagent rewrites the five `docs/examples/core/ObjectMap/` examples below and blesses
their `.out` files, which must print what they print today. The session writes the rest.

- [ ] The feature pages that state the old answer:
  `docs/examples/lang/iteration/what-foreach-walks/about.md:6`,
  `docs/examples/lang/iteration/the-two-interfaces/about.md:1` and `:10`,
  `docs/examples/lang/iteration/core-collections-are-iterable/about.md:3` and `details.md:5`,
  `docs/examples/lang/statements/foreach/details.md:10`,
  `docs/examples/lang/types/every-binding-has-a-type/about.md:3`,
  `docs/examples/lang/types/array-t/details.md:5` (the key of a loop over an array).
- [ ] `docs/reference/core/ObjectMap.md:11`: the introduction says a loop with a key gives each key with
  its value and a loop without one gives the keys, and its example's loop at `:34` walks the pairs, with
  its output unchanged.
- [ ] The `docs/examples/core/ObjectMap/` examples whose loop reads `get` for the key it is on use the
  keyed form: `isEmpty/03-save-only-when-something-changed.nvs:41`, `remove/01-remove-an-entry.nvs:28`,
  `remove/03-track-running-jobs.nvs:32`, `set/01-add-and-replace-a-value.nvs:26`,
  `set/03-count-visits-per-user.nvs:25`.
- [ ] `website/src/content/docs/syntax/control-flow/foreach.mdx:1`: `covers` gains
  `lang:iteration/keys-and-values-from-a-map`, a `FeatureSection` for it after *Core collections*, and
  the bullet at `:32` says that a key works over an array and a `KeyedIterable`.
  `website/src/content/docs/syntax/control-flow/iterators.mdx:1`: `covers` gains `KeyedIterable` and
  `KeyedIterator`, the opening at `:9` names them, and a table of the keyed methods follows the one at
  `:17`. `website/src/content/docs/syntax/basics/variables.mdx:1` takes the new key sentence through its
  feature's `about.md`.
- [ ] `website/src/content/docs/in-depth/what-changed.mdx:1`: the `new` entry in the wording of
  § *Standing decisions*, in the form the `website-follows-changes` rule gives. Every page covering a
  changed feature is restamped as that rule says. The `site-changes` check run once.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.
- [ ] The newcomer review the `website-follows-changes` rule names, over every page, card and diagnostic
  this goal changed, each finding fixed and each page stamped. The `site-review` check run once.

## Handoff

### State

Written on 2026-11-17 from a conversation with the person. Their calls: a key and value `foreach` over a
map without writing types (`var`), keeping `[]` array-only for the reasons in § *Standing decisions*,
and the plan as a whole. The authoring session's calls: the interface names and `iterateKeyed`, keeping
the single-binding form's meaning, the new rule fragment in place of renaming
`rule:iteration/two-interfaces`, the feature's section name, the editor item, and the attack's shape.
Nothing is built yet.

### Next

Stage 1, once its gate holds: `docs/goals/object-collections-fast.md` does not exist. Start with the
rule at `docs/rules/iteration/keyed-interfaces.md:1`, then the roster at
`crates/nvs-hir/src/interfaces.rs:45` and the classification at
`crates/nvs-types/src/expr/iteration.rs:158`.

No test is expected to go red, apart from `docs/examples/lang/iteration/the-two-interfaces/04-an-iterator-has-no-key.out`
and `tests/conformance/iter/a-generator-has-no-key-to-bind.nvst`, whose `E0444` help line stage 1
changes.

### Backlog
