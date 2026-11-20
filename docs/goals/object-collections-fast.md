---
looper: 1
title: ObjectMap and ObjectSet look up in one hash and start a loop without a copy
summary: "Reached when a Core\\ObjectMap or Core\\ObjectSet whose key type is exactly string, int or a class stores the key directly so get and has cost one hash lookup, a foreach over either shares the store instead of copying it, the behaviour every existing case pins is unchanged, and a bench shows both against a plain array."
context: [crates/nvs-stdlib/src/identity_store.rs, crates/nvs-stdlib/src/objmap.rs, crates/nvs-stdlib/src/objset.rs, docs/rules/core-api/no-methods-on-scalars-or-arrays.md]
stages:
  - id: 1
    title: the facts the keys rest on are confirmed, and the benches measure today's cost
    status: done
    context: [crates/nvs-runtime/src/identity.rs, crates/nvs-runtime/src/value.rs, benches/members/README.md, benches/members/lang/iteration/core-collections-are-iterable.nvs, docs/rules/testing/member-perf-ledger.md, docs/rules/enums/representation.md]
  - id: 2
    title: a map or set keyed by exactly string or int stores the key as it is
    status: done
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/expr_table.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-runtime/src/array.rs, docs/rules/types/arrays.md]
  - id: 3
    title: a map or set keyed by a class stores each object by its identity
    status: done
    context: [crates/nvs-runtime/src/identity.rs, crates/nvs-types/src/expr/calls.rs, docs/rules/expressions/equality-semantics.md, tests/conformance/core/object-collections-follow-an-object-but-copy-a-value.nvst]
  - id: 4
    title: a loop shares the collection's store and copies it only on a write
    status: done
    context: [crates/nvs-stdlib/src/cursor.rs, crates/nvs-stdlib/src/heap.rs, docs/rules/iteration/two-interfaces.md, docs/rules/classes/two-copy-depths.md, tests/conformance/core/object-collections-walk-a-snapshot-so-a-body-may-mutate-what-it-walks.nvst]
  - id: 5
    title: the figures after, the trade-offs and the pages
    status: done
    context: [docs/reference/core/ObjectMap.md, docs/reference/core/ObjectSet.md, website/src/data/changes.json, docs/agent/prose.md, docs/rules/core-api/reference-card.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: keying
    stage: 2
    name: "the checker keys a new ObjectMap or ObjectSet by its first type argument: exactly string, exactly int, or the chain for every other type"
    run: [cargo, test, --test, generics, --, a_new_object_map_is_keyed_by_its_exact_key_type, a_new_object_set_is_keyed_by_its_exact_element_type]
    timeout: 1800
    tests:
      - a_new_object_map_is_keyed_by_its_exact_key_type
      - a_new_object_set_is_keyed_by_its_exact_element_type
  - id: direct-keys
    stage: 2
    name: "an int key comes back as an int, a string key comes back byte for byte, and an int and its decimal string are two keys of a map keyed by int|string"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/object-map-an-int-key-comes-back-as-an-int.nvst, --case, tests/conformance/core/object-map-string-keys-come-back-byte-for-byte.nvst, --case, tests/conformance/core/object-map-an-int-and-its-decimal-string-are-two-keys-of-a-union-keyed-map.nvst, --case, tests/conformance/core/object-set-an-int-member-comes-back-as-an-int.nvst]
    timeout: 1800
    tests:
      - tests/conformance/core/object-map-an-int-key-comes-back-as-an-int.nvst
      - tests/conformance/core/object-map-string-keys-come-back-byte-for-byte.nvst
      - tests/conformance/core/object-map-an-int-and-its-decimal-string-are-two-keys-of-a-union-keyed-map.nvst
      - tests/conformance/core/object-set-an-int-member-comes-back-as-an-int.nvst
  - id: object-keying
    stage: 3
    name: "a new collection keyed by a class is keyed by identity, and one keyed by a type that can hold a value that is not an object keeps the chain"
    run: [cargo, test, --test, generics, --, a_new_collection_keyed_by_a_class_is_keyed_by_identity, a_collection_keyed_by_a_type_with_a_value_that_is_no_object_keeps_the_chain]
    timeout: 1800
    tests:
      - a_new_collection_keyed_by_a_class_is_keyed_by_identity
      - a_collection_keyed_by_a_type_with_a_value_that_is_no_object_keeps_the_chain
  - id: object-keys
    stage: 3
    name: "a new object is never found under the key of an object that was removed, and a subclass instance is a key of a map keyed by its parent"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/object-map-a-new-object-is-never-found-under-a-removed-objects-key.nvst, --case, tests/conformance/core/object-set-a-new-object-is-never-a-member-in-place-of-a-removed-one.nvst, --case, tests/conformance/core/object-map-a-subclass-instance-is-a-key-of-a-map-keyed-by-its-parent.nvst]
    timeout: 1800
    tests:
      - tests/conformance/core/object-map-a-new-object-is-never-found-under-a-removed-objects-key.nvst
      - tests/conformance/core/object-set-a-new-object-is-never-a-member-in-place-of-a-removed-one.nvst
      - tests/conformance/core/object-map-a-subclass-instance-is-a-key-of-a-map-keyed-by-its-parent.nvst
  - id: cursor-shares
    stage: 4
    name: "a cursor holds the collection's own store until a write separates the collection, and a heap's cursor walks a sorted copy of its own"
    run: [cargo, test, --lib, --, a_cursor_shares_its_collections_store_until_a_write_separates_it, a_heap_cursor_walks_a_sorted_copy_of_its_own]
    timeout: 1800
    tests:
      - a_cursor_shares_its_collections_store_until_a_write_separates_it
      - a_heap_cursor_walks_a_sorted_copy_of_its_own
  - id: loops-share
    stage: 4
    name: "a collection changed inside its loop is unchanged for the loop and changed after it, and a removal inside the loop lets it finish over every original entry, for every keying"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/iter/a-core-collection-changed-inside-its-loop-is-changed-after-it-in-every-keying.nvst, --case, tests/conformance/iter/a-removal-inside-a-loop-over-a-core-collection-lets-it-finish-in-every-keying.nvst]
    timeout: 1800
    tests:
      - tests/conformance/iter/a-core-collection-changed-inside-its-loop-is-changed-after-it-in-every-keying.nvst
      - tests/conformance/iter/a-removal-inside-a-loop-over-a-core-collection-lets-it-finish-in-every-keying.nvst
  - id: loop-start-constant
    stage: 4
    name: "starting a foreach over an ObjectMap or ObjectSet costs the same counts at 12,500 entries and at 100,000"
    run: [bun, nv, scaling, --iterations, --check, lang/iteration/core-collections-are-iterable]
    stdout_contains: ["every bench costs the same per operation in every batch"]
    timeout: 1800
  - id: pages
    stage: 5
    name: "no page is stale, the change has its changes.json entry, and every changed user text has its review stamp"
    run: [bun, nv, site, --check, stale, changes, review]
    timeout: 900
---

# ObjectMap and ObjectSet look up in one hash and start a loop without a copy

## Why

This is the first of three goals that build revocable objects. `keyed-iteration` and
`revocable-objects` follow it on the chain. A revocable object is found through an
identity-keyed collection, so the person's requirement is that `Core\ObjectMap<K, V>` and
`Core\ObjectSet<T>` stay fast for large collections, and that a read costs what a plain array read
costs. Nothing a program writes or sees changes.

Today both classes sit on the chain store of `crates/nvs-stdlib/src/identity_store.rs:1`: an
`NvsArray` keyed `"<16 hex digits of the identity hash>#<n>"`. `locate` (`:104`) hashes the value
with `nvs_runtime::value_hash`, writes a chain key, looks it up (the `NvsArray` SipHashes that key
again) and compares with `value_identical`. A map holds two stores, keys and values, under one set of
chain keys (`crates/nvs-stdlib/src/objmap.rs:4`), so `get` is about three hashes and two lookups
where `$a[$k]` is one. And every `foreach` copies the elements into a fresh list
(`crates/nvs-stdlib/src/cursor.rs:22`): one allocation and one reference per element, per loop.

The goal keeps the chain where it is needed and removes it where the key type makes it redundant:
a `string` or `int` key goes straight into the array, an object goes in by its identity, and a loop
shares the store the way a `foreach` over an array shares the array.

## Standing decisions

**These are the person's calls, settled in the conversation that wrote this goal. Do not reopen
them.** A session that finds one cannot hold follows its fallback, records why in the home named,
and goes on.

**No syntax and no observable behaviour changes.** Every existing case under
`tests/conformance/core/object-collections-*.nvst`, `object-map-*.nvst`, `object-set-*.nvst`,
`objectset-*.nvst`, `tests/conformance/iter/` and `arr-from-takes-a-core-collection.nvst` passes
with its expected output unedited. A mode that cannot keep one of them green falls back to the
chain for that key type, and the fallback is recorded in the module doc.

**A collection's keying is chosen once, at construction, and stored in the instance (the person's
call; how it is passed is the authoring session's call).** Four keyings:

| Keying | When the first type argument is | The store's key | `get` and `has` |
|---|---|---|---|
| `string` | exactly `string` | the string itself | one lookup |
| `int` | exactly `int` | the `int`, normalised to its decimal key as an array subscript is (`rule:types/arrays`) | one lookup, with no key string built while the array is a list |
| `object` | a class whose every runtime value is an object (stage 3 decides which types) | the object's identity, written on the stack | one lookup |
| `chain` | anything else: `mixed`, a union, a nullable, an enum, a shape, an array, a type parameter, a qualified `tainted string` or `secret string` | today's chain key | today's cost |

- The first type argument decides for both classes: `K` of `ObjectMap<K, V>` and `T` of
  `ObjectSet<T>` (`crates/nvs-stdlib/src/registry.rs:4147` `GENERIC_CLASSES` names them in that
  order). The type is read after aliases are resolved.
- **The chain is correct for every key type**, so it is the safe default everywhere: a construction
  path the checker does not classify, a `new $cls()` through a class reference, and any type a stage
  finds it cannot prove builds a `chain` collection.
- **How the keying reaches the constructor: one symbol per keying.** The checker records the keying
  on `ExprInfo::New` (`crates/nvs-types/src/expr_table.rs:567`, a new field, `chain` for every other
  class) where `check_new_type_args` (`crates/nvs-types/src/expr/calls.rs:1741`) binds the written
  arguments. `nvs_types::core_constructor_symbol` (`crates/nvs-types/src/lib.rs:278`, which
  re-exports `crates/nvs-stdlib/src/registry.rs:3156`) takes the keying and answers one of four
  symbols per class: today's `nvs_core_object_map_new` is the `chain` one, and
  `nvs_core_object_map_new_string`, `_int` and `_object` join it, and the same for
  `nvs_core_object_set_new`. `crates/nvs-ir/src/lower/expr.rs:3784` passes the recorded keying and
  emits the same one `CoreCall` it emits today. Each symbol writes its keying into a `keying` slot.
  Why: no hidden argument, no change to the constructor's checked arity (`params: &[]`, `args: [0]`),
  nothing new at run time but the slot read, and `crate::identity_store` stays the one module that
  decides what a key is. The keying type is an enum in `identity_store`, made public through
  `nvs_stdlib::registry` because `nvs-types` depends on `nvs-stdlib`. Declined: a constructor
  argument the IR adds, because it changes the checked signature of a constructor that takes
  nothing and costs a value per `new`.
- **Every operation that reads two collections reads each by its own keying.** `union`,
  `intersect` and `diff` (`crates/nvs-stdlib/src/objset.rs:294` `collect_into`) look a member up in
  each set with that set's keying, and their result takes the receiver's keying. Two sets of the
  same static type can differ in keying when one was built through a path the checker leaves on the
  chain.

**What each keying stores.** A `string` or `int` map holds one store: the values, under the key
itself. The key store is gone, and `keys()` and a `foreach` read the keys back from the array, as
`int`s for an `int` map. A `string` or `int` set holds one store with each member as the value
under its own key. An `object` map keeps two stores, keys and values, under the identity key, and
`get` and `has` read only the values store. An `object` set holds one store. A `chain` collection is
today's layout exactly.

**An object key is its identity, and the collection's reference keeps it unique.** The map or set
retains the object for as long as the entry exists, so no other object can have that identity
while the entry is there. The key is written on the stack in a form that can never read as a
decimal integer, so the array never takes it for an `int` subscript. The key never leaves the store:
`keys()`, `values()`, a `foreach`, a dump and every error read the stored values, never the keys.
Security and request isolation are not spent: no address reaches a program or a record. Fallback
(the person's call): if stage 1 finds that the runtime can move an object or that object identity is
not its address, `object` keying is not built, objects keep the chain, and the identity-store doc
records why.

**A loop shares the store instead of copying it (the person's call).** `iterate()` takes one
reference on the store's `NvsArray` and the cursor walks it by slot (`NvsArray::next_slot`,
`crates/nvs-runtime/src/array.rs:1930`). A write during the loop separates the collection once,
through the copy-on-write every `NvsArray` mutator already makes (`make_unique`, `:2041`), so the
loop still walks the collection as it was when it began. That is the documented behaviour, and it
is unchanged. A `foreach` over a map still yields its keys, and over a set its members. **`Core\Heap`
keeps its copy** (the authoring session's call): its storage order is not its pop order, so its
cursor walks the sorted list `crates/nvs-stdlib/src/heap.rs:519` `sorted` builds, as today. The
cursor learns what it yields from a slot: the value at the slot, the key as a `string`, or the key
as an `int`.

**What it spends, and the trade-offs** (decision 8, written into the module docs in stage 5):

- Performance only improves. `get`, `has`, `set` and `remove` on a `string`, `int` or `object`
  collection are one array lookup, with no identity hash and no chain walk. A loop start is one
  reference count, not a copy. A write inside a loop copies the store once, which is the cost the
  loop paid on every start before.
- Memory goes down. A `string` or `int` map drops its key store: one array and one entry per pair
  fewer. Every loop drops its list: one allocation and one reference per element. All of it stays
  charged to the request that built the collection.
- No user-visible change: the same syntax, results, order and errors.
- The implementation gains three keyings in one module. `identity_store` stays the one place a key
  is chosen, and each class reads its keying from one slot.

**The bench (the authoring session's call on sizes).** The comparison the person asked for is made
with the bench tree's own devices (`benches/members/README.md`): the counts decide and the clock is
reported.

- `benches/members/lang/iteration/core-collections-are-iterable.nvs` is rewritten: each round starts
  a `foreach` over an `ObjectMap<string, Item>` and an `ObjectSet<Item>`, reads the first element and
  breaks, then walks an eight-entry `Core\Heap<int>` as today. It declares `complexity constant`.
  Its `.scale.nvs` sibling declares `// bench: rounds 64`, `start 12500` and `max 100000`, and grows
  the map and the set while the heap stays at eight. A new `.twin.nvs` does the same over `array<Item>`
  values. This is the count check: today a loop start copies the collection, so its bytes grow with
  the size and the ramp fails `constant`.
- `benches/members/core/ObjectMap/get.nvs`, `has.nvs` and `set.nvs` are rewritten to
  `ObjectMap<string, Item>`, and `benches/members/core/ObjectSet/has.nvs` keeps `ObjectSet<Key>`. Each
  gets a `.twin.nvs` doing the same read over `array<Item>` (the set's twin keyed by the `int` id)
  and a `.scale.nvs` with `rounds`, `start 12500` and `max 100000`. The twin's ratio is the "costs
  what an array costs" figure, reported and never a gate.
- Each rewritten bench stays within its share of `data/perf/bench-budget.json` at its own N, so the
  collection inside it stays small, and the 100,000 entries are the sibling's. If `bun nv scaling
  --budget` counts a sibling too, its `max` drops to the largest size under the share, and § State
  says so.

**Where it is written down.** The keyings, the identity key and what each spends:
`crates/nvs-stdlib/src/identity_store.rs:1`'s module doc, as Decision sections. The loop:
`crates/nvs-stdlib/src/cursor.rs:22`'s Decision, rewritten as a whole. No rule fragment states the
chain or the copy (a Grep of `docs/rules/` for `snapshot`, `ObjectMap` and `ObjectSet` finds none),
so no rule changes. A rule a session finds that does state either is rewritten in the same slice.

**The website (the rules of `website-follows-changes`).** The change gets one highlight
entry in `website/src/data/changes.json`, which the release puts in its release notes. `docs/reference/core/ObjectMap.md` and `docs/reference/core/ObjectSet.md`, and every page
`bun nv site --stale` names, are reread and restamped. Every code comment and doc that describes the
old copy or the old chain is rewritten as a whole.

**What stage 1 found.** These are the facts stages 2 and 3 build on.

- **An object never moves, and its identity is its address.** `NvsObj::alloc` places it once and
  `dismantle` frees it in place. The one change of owner, `crate::graph`'s crossing, hands the same
  pointer over and relinks it (`crates/nvs-runtime/src/object.rs:202` § *every object is on its
  context's live list*). `collect` frees only cyclic garbage, never an object a live collection
  holds. `crates/nvs-runtime/src/sweep.rs` deletes temporary directories and touches no object.
  `hash_shallow` hashes an object in its `_` arm by the tag byte and `value.bits()`
  (`crates/nvs-runtime/src/identity.rs:355`), and `shallow_identical`'s fall-through compares tag
  and bits (`:244`). So `object` keying is built in stage 3.
- **The static types whose every value is an object:** `Ty::Class` (a class, an interface and every
  `Core` class, `Core\Html\Markup` included: it is a one-slot instance), `Ty::Object`, `Ty::Shape`
  (every shape is a subtype of `object`, `rule:types/object-top`), `Ty::Callable` and
  `Ty::CallableSig` (a callable is an object, `crates/nvs-runtime/src/value.rs:72`), and a
  `Ty::Intersection` whose members are all of these. An enum implements no interface (`E0218`,
  `rule:enums/no-class-machinery`), so no enum case reaches an interface type. Kept on the chain:
  `Ty::Iterable` (an array is one), `Ty::ClassRef`, `Ty::ShapeOfCallables`, a type variable, and
  every union, nullable included (`crates/nvs-types/src/ty.rs:37`).
- **Typed code passes an `int` key only as `Tag::Int` and a `string` key only as `Tag::Str`.**
  `assignable` (`crates/nvs-types/src/expr/assign.rs:262`) widens `int` and `uint` only to `float`,
  an enum case only to its enum, and a single-value type only to its base. An enum case reaches
  `int` only through `as int`. **But an erased view gets past it:** `as` and `is` check a
  collection's class and never its type arguments. A probe built an `ObjectMap<string, int>`, put
  `"5"` in it and called `set(5, …)` through `$any as Core\ObjectMap<int, int>`, and it printed a
  count of `2`. The same held through `is`. The next decision says what a keyed collection does then.
- **`tainted` and `secret` leave nothing at run time.** No tag and no string header field carries
  either (`crates/nvs-runtime/src/value.rs`, `crates/nvs-runtime/src/string.rs`).
- **A store key is shown by two readers.** `Core\Debug::dump` prints each slot's array with its
  chain keys (`$keys => array(1) [ "700af0ec7d4a7e00#0" => string(1) "a" ]`, and `$entries` for a
  set). An uncaught-error record walks a thrown object's properties with the same generic object
  walk (`crates/nvs-runtime/src/record.rs:410` `properties_of`), so a collection held in a property
  shows the same. `Core\Json::encode` throws (the class has no JSON encoding) and
  `Core\Serialize::encode` throws (`cannot be written to bytes`), so neither shows a key. No case
  under `tests/` pins a collection's dump. So stage 3's printer item holds: both printers show the
  stored values only, with a case each, before `object` keying lands.

**A key whose tag is not the keying's turns the collection into a `chain` one, once (stage 1's
session's call).** Stage 1 found that `$any as Core\ObjectMap<int, int>` and `$any is
Core\ObjectMap<int, int>` check the class and never its type arguments, so a map built
`<string, int>` takes `set(5, …)` through that view, and today it then holds `"5"` and `5` as two
keys. So the instance's keying is a fact about the instance, never a promise the static type keeps.
Each `string`, `int` or `object` operation compares the key's tag with its keying first, one byte
compare. On a mismatch it rebuilds its store under chain keys, writes `chain` into the `keying`
slot, and goes on as a `chain` collection, so every result is today's. A read that misses on the
tag (`get`, `has`) can answer from the tag alone without the rebuild, since no stored key has that
tag. Why: it keeps § *No syntax and no observable behaviour changes* on a path the checker cannot
see, and nothing is spent on the typed path but the compare. Declined: throwing on the mismatch,
which changes what a program that works today does; and checking type arguments in `as` and `is`,
which is a language change this goal does not make. Stage 2 adds a conformance case for the erased
view, and stage 3 the same for an object keying.

**Lanes.** One lane. Every stage edits `crates/nvs-stdlib/src/identity_store.rs`, `objmap.rs` or
`objset.rs`, so the stages run in order. Stage 2 edits `crates/nvs-types/src/expr/calls.rs` and
`crates/nvs-ir/src/lower/expr.rs`, so this goal does not run as a side goal beside a compiler goal
that edits them.

**Limits: what stops this goal.** Counts decide and never the clock. One attempt per item. An item
still red after its attempt goes into the handoff with its cause, and the stage moves on to its
other items. Each stage gets at most two sessions, counted in § State. A stage still open after its
second session stops the loop with `BLOCKED` naming the item. No item is added to a stage, and no
check is loosened or renamed to pass. A keying that breaks an existing case takes its fallback (the
chain) instead of an edited expected output.

## Stages

### 1. the facts the keys rest on are confirmed, and the benches measure today's cost

**Gate:** `docs/goals/website-follows-changes.md` does not exist.

**Does:** confirms what the `object`, `int` and `string` keyings rely on before any code depends on
it, and measures today's cost with the benches that later stages are judged by. Nothing in a crate
changes in this stage.

**Split:** one writing subagent writes the bench files of the last two items, with
`benches/members/README.md` as its brief and the file list of § *The bench*. The session confirms the
facts, then runs the before measurement once the subagent ends.

- [x] No object moves: read `crates/nvs-runtime/src/object.rs:3423` `ObjHeader` and the paths that
  free or reclaim an object (`crates/nvs-runtime/src/release.rs:1`, `crates/nvs-runtime/src/sweep.rs:1`),
  and confirm that an object's address is fixed from allocation to free. Confirm that
  `crates/nvs-runtime/src/identity.rs:326` `hash_shallow` hashes an object by `value.bits()` and that
  `value_identical` (`:150`) compares two objects by their pointer (`rule:expressions/equality-semantics`).
  Write the finding into § State. If either fails, § *An object key is its identity*'s fallback
  holds for stage 3.
  - [x] the allocation, the crossing and the collector read
  - [x] the hash and the comparison read
  - [x] the finding written into § State
- [x] Which static types hold only objects at run time: a class, an interface (an enum may implement
  one, and an enum case is `Tag::EnumInt` or `Tag::EnumUint`, `crates/nvs-runtime/src/value.rs:88`),
  a callable type, `object` if the language has it. Write the list into § State; stage 3 keys only
  those by identity (`rule:enums/representation`).
  - [x] `Ty`'s variants and the `object` and shape rules read
  - [x] the list written into § State
- [x] An `int` parameter receives only `Tag::Int` and a `string` parameter only `Tag::Str`: an
  int-backed enum case, a `uint`, a `float` and a `bytes` each need a conversion first. Confirm it
  from the checker's assignability and the case corpus, and write it into § State. If an enum case can
  reach an `int` parameter as itself, `int` keying is not built and `int` keys keep the chain
  (`rule:enums/representation`, `rule:types/arrays`).
  - [x] the checker's `assignable` read
  - [x] an erased view probed with `as` and `is`, and the decision recorded
  - [x] the finding written into § State
- [x] No store key is ever shown. Read what `Core\Debug::dump`, `Core\Serialize`
  (`crates/nvs-stdlib/src/instance.rs:345`, `Slots::NotReadBack`), `Core\Json` and an uncaught-error
  record (`crates/nvs-runtime/src/record.rs:434`) print for an `ObjectMap` and an `ObjectSet`, and
  write it into § State. If any of them prints a slot's array, stage 3 makes it print the stored
  values only and adds a case for it before `object` keying lands
  (`rule:classes/serialize-is-a-closed-format`, `rule:errors/record-transformations`).
  - [x] dump, JSON and Serialize probed on a map and a set
  - [x] the record's object walk read
  - [x] the finding written into § State
- [x] The benches of § *The bench*, written by the subagent: rewrite
  `benches/members/lang/iteration/core-collections-are-iterable.nvs:1` and its `.scale.nvs`, and
  `benches/members/core/ObjectMap/get.nvs:1`, `has.nvs:1`, `set.nvs:1`; write the twins and scale
  siblings beside them and beside `benches/members/core/ObjectSet/has.nvs:1`. Each top comment
  follows `AGENTS.md` § *Text an end user reads*, and `bun nv proofs --comments <paths>` passes.
  `bun nv scaling --budget` is green, or each over-share sibling's `max` is lowered as § *The bench*
  says (`rule:testing/member-perf-ledger`).
  - [x] fourteen files written, and each runs
  - [x] `bun nv proofs --comments` passes on all of them
  - [x] no bench of this goal is over its share; `bun nv scaling --budget` is red on 48 other benches
- [x] The before figures: `bun nv scaling --iterations lang/iteration/core-collections-are-iterable
  core/ObjectMap/get core/ObjectMap/has core/ObjectMap/set core/ObjectSet/has` once. It fails on the
  iteration bench, and that failure is the figure: write each bench's allocations and bytes per
  operation and its clock at 100,000 entries into § State. Then `bun nv proofs --record-perf --id
  'Core\ObjectMap::get'`, and the same for `Core\ObjectMap::has`, `Core\ObjectMap::set` and
  `Core\ObjectSet::has`, and write each `ns/op` and `twin_ratio` into § State
  (`rule:testing/member-perf-ledger`). If a guard refuses a run, name the call in the handoff and go on.
  - [x] the `--iterations` run: 8 flat and 2 unclear, with every `.scale.nvs` skipped
  - [x] the four members recorded with `--force` and the release binary
  - [x] the iteration bench measured: it fails `constant` on bytes, which is the figure
  - [x] the figures written into § State

### 2. a map or set keyed by exactly string or int stores the key as it is

**Does:** the checker chooses the keying, `new` lowers to the keyed symbol, and the two classes
store a `string` or `int` key directly, as § *A collection's keying* and § *What each keying stores*
say.

**Split:** at the session's start, one writing subagent writes the four `.nvst` cases the
`direct-keys` check names, in the shape of
`tests/conformance/core/object-map-keys-on-identity.nvst`. They pass on today's tree, which is the
point: the session runs the check once before the change and once after. The session writes the
code.

- [x] The keying, chosen by the checker: the enum in `crates/nvs-stdlib/src/identity_store.rs:1`,
  public through `crates/nvs-stdlib/src/registry.rs:3134`; the field on `ExprInfo::New`
  (`crates/nvs-types/src/expr_table.rs:567`), set where `check_new_type_args`
  (`crates/nvs-types/src/expr/calls.rs:1741`) binds the arguments, from the first one: exactly
  `string`, exactly `int`, else `chain` (`rule:types/arrays`).
- [x] The symbols: `crates/nvs-stdlib/src/registry.rs:3156` `constructor_symbol` takes the keying;
  `crates/nvs-ir/src/lower/expr.rs:3784` passes the recorded one. The keyed symbols join
  `crates/nvs-stdlib/src/objmap.rs:300` and `crates/nvs-stdlib/src/objset.rs:261` `address`, the
  roster at `crates/nvs-stdlib/src/lib.rs:362`, and `member_label` (`registry.rs:3168`), which names
  each as the class's constructor (`rule:testing/debug-probes`).
- [x] `crates/nvs-stdlib/src/identity_store.rs:104` `locate` and `:124` `vacate` take the keying: a
  `string` key is the string's own bytes, an `int` key goes through `NvsArray::get_index`/`set_index`
  (`crates/nvs-runtime/src/array.rs:1497`, `:1507`), and neither walks a chain. `borrow`, `edit` and
  `replace` keep their shape, because `crates/nvs-stdlib/src/hash.rs:1215` and
  `crates/nvs-stdlib/src/compress.rs:1459` use them too (`rule:expressions/equality-semantics`).
- [x] `Core\ObjectMap` in the two direct keyings (`crates/nvs-stdlib/src/objmap.rs:172` slots,
  `:346` new, `:365` set, `:420` get, `:443` has, `:457` remove, `:499` keys, `:535` clear): one store,
  and `keys()` reads the keys back, as `int`s through `NvsArray::slot_key` (`array.rs:1943`) for an
  `int` map. `count`, `isEmpty` and `values` read the one store (`rule:core-api/no-methods-on-scalars-or-arrays`).
- [x] `Core\ObjectSet` in the two direct keyings (`crates/nvs-stdlib/src/objset.rs:135` slots,
  `:362` new, `:375` add, `:400` has, `:413` remove), and the algebra (`:294` `collect_into`, `:327`
  `built_from`, `:461` union, `:473` intersect, `:487` diff) reads each operand by its own keying, as
  § *Every operation that reads two collections* says.
- [x] The tests: `crates/nvs-types/tests/generics.rs:1` gains the two tests the `keying` check names,
  covering `string`, `int`, an alias of `string`, `?string`, `int|string`, `mixed`, `uint`,
  `tainted string` and `array<int>`. Run the `keying` and `direct-keys` checks once, and every
  existing case named in § *No syntax and no observable behaviour changes* once with `bun nv suite
  tests/conformance/core/` (`rule:testing/feature-proofs`).

### 3. a map or set keyed by a class stores each object by its identity

**Does:** the `object` keying of § *An object key is its identity*, for exactly the types stage 1
listed. If stage 1 found that objects can move or that identity is not the address, this stage
records the fallback in the module doc, writes the cases and tests against the chain, and builds
nothing else.

**Split:** one writing subagent writes the three `.nvst` cases the `object-keys` check names, in the
shape of `tests/conformance/core/object-collections-follow-an-object-but-copy-a-value.nvst`. The
session writes the code.

- [x] The keying: `crates/nvs-types/src/expr/calls.rs:1741` chooses `object` for a first type
  argument on stage 1's list, and `chain` for an enum, a nullable class, an interface an enum can
  implement, and every type not on the list (`rule:expressions/equality-semantics`).
- [x] The key: `crates/nvs-stdlib/src/identity_store.rs:76` beside `ChainKey`, an identity key
  written on the stack from the object's pointer, in a form that never reads as a decimal integer,
  and `locate`/`vacate` (`:104`, `:124`) for it with no hash of the value and no chain
  (`rule:expressions/equality-semantics`).
- [x] `Core\ObjectMap` (`crates/nvs-stdlib/src/objmap.rs:332` `at`, `:420` get, `:443` has, `:457`
  remove) and `Core\ObjectSet` (`crates/nvs-stdlib/src/objset.rs:375` add, `:400` has, `:413` remove)
  in the `object` keying. `get` and `has` read only the values store
  (`rule:core-api/no-methods-on-scalars-or-arrays`).
- [x] If stage 1 found a printer that shows a slot's array, it shows the stored values only, with a
  case beside the three below (`rule:errors/record-transformations`).
- [x] The tests: the two tests the `object-keying` check names in
  `crates/nvs-types/tests/generics.rs:1`, and the three cases the `object-keys` check names. The
  removed-object cases free many objects after a removal and make new ones, so a new object can take
  a freed address, and assert that none of them is found. Run both checks once, and `bun nv suite
  tests/conformance/core/` once (`rule:expressions/equality-semantics`).

### 4. a loop shares the collection's store and copies it only on a write

**Does:** § *A loop shares the store instead of copying it*. `Core\Heap` keeps its copy.

**Split:** one writing subagent writes the two `.nvst` cases the `loops-share` check names, in the
shape of `tests/conformance/core/object-collections-walk-a-snapshot-so-a-body-may-mutate-what-it-walks.nvst`,
each covering a `string`, an `int`, an `object` and a `chain` collection of both classes. The
session writes the code.

- [x] `crates/nvs-stdlib/src/cursor.rs:68` the slots and `:102` `over`: the cursor holds a
  reference on the store and a slot that says what it yields (the value, the key as a `string`, the
  key as an `int`). `:144` `stepped` and `:158` `read` walk the shared store by slot, and `read`
  builds an `int` key through `NvsArray::slot_key` with no allocation (`rule:iteration/two-interfaces`).
- [x] `iterate` in `crates/nvs-stdlib/src/objmap.rs:523` and `crates/nvs-stdlib/src/objset.rs:502`:
  one retain on the store the keying walks (the key store of a `chain` or `object` map, the one store
  otherwise), and no `listed`. `crates/nvs-stdlib/src/identity_store.rs:241` `listed` stays for
  `keys()` and `values()`. Confirm by reading that every mutator the classes call (`set`,
  `overwrite`, `unset`, `rename`, `set_index`) separates a shared array first
  (`crates/nvs-runtime/src/array.rs:2041`), and that `clear` (`identity_store.rs:230` `replace`) puts a
  fresh array in the slot and leaves the cursor's alone (`rule:classes/two-copy-depths`).
- [x] `crates/nvs-stdlib/src/heap.rs:759` `iterate` passes its sorted list to the new `over` with
  the value yield, and its cursor walks a list that nothing else holds (`rule:iteration/foreach-subjects`).
- [x] `crates/nvs-stdlib/src/cursor.rs:22`'s Decision is rewritten as a whole: the cursor shares the
  store, a write separates the collection once, the loop sees the collection as it was when it
  began, why `Core\Heap` keeps its copy, and what it spends now (one reference per loop, and one copy
  of the store when the body writes). `crates/nvs-stdlib/src/cursor.rs:1`'s first line,
  `crates/nvs-stdlib/src/objmap.rs:29` and `:78`, and `crates/nvs-stdlib/src/objset.rs:43` stop
  calling it a snapshot copy (`rule:iteration/two-interfaces`).
- [x] The tests: `crates/nvs-stdlib/src/cursor.rs:201` gains the two tests the `cursor-shares` check
  names. The first asserts that during a loop the cursor and the map hold the same array, and that
  after a `set` the map holds another one and the cursor still holds the first. Write them, and the
  two cases, then run the `cursor-shares` and `loops-share` checks once, `bun nv suite
  tests/conformance/iter/ tests/conformance/core/` once, and the `loop-start-constant` check once.
  Add `// bench: allocations N` to the iteration bench with the count the ramp printed
  (`rule:testing/member-perf-ledger`).

### 5. the figures after, the trade-offs and the pages

**Does:** measures the result against stage 1's figures, writes the trade-offs into the module
docs, and brings the pages up to date under the website rules of `website-follows-changes`.

- [x] The after figures: the same `bun nv scaling --iterations ...` and the four `bun nv proofs
  --record-perf --id ...` runs as stage 1, plus `bun nv proofs --record-perf --id
  'lang:iteration/core-collections-are-iterable'`. Write each figure beside its before figure in
  § State (`rule:testing/member-perf-ledger`).
  - [x] every stale record listed and recorded on a release binary of the tree
  - [x] the iteration bench recorded and its `--iterations --check` run
  - [x] the figures written into § State
- [x] `crates/nvs-stdlib/src/identity_store.rs:1`'s module doc, rewritten as a whole: one Decision
  section per keying with its key and what it spends, the Decision on how the keying reaches the
  constructor with its Declined alternative, and the four trade-off lines of § *What it spends, and
  the trade-offs*. `crates/nvs-stdlib/src/objmap.rs:4`'s Decision says which keyings keep two stores
  and which keep one (`rule:core-api/no-methods-on-scalars-or-arrays`).
  - [x] the module doc rewritten with a keying table, a Decision per keying, the constructor
    Decision and the trade-offs
  - [x] `objmap.rs`'s Decision names the two layouts
- [x] `docs/reference/core/ObjectMap.md:11` and `docs/reference/core/ObjectSet.md:9`: reread, keep
  the sentence about what a loop sees, and add one sentence each in end-user prose, such as "A
  `foreach` does not copy the map. If the loop changes the map, the loop still walks the entries the
  map had when it began." Restamp both, and every page `bun nv site --stale` names
  (`rule:core-api/reference-card`).
  - [x] one sentence on each page
  - [x] the six stale about pages and help cards reread and stamped
- [x] The highlight entry in `website/src/data/changes.json:1`, in the form the rule
  `website-follows-changes` wrote gives it (Grep `docs/rules/` for `changes.json`). In end-user prose, such as: "`Core\ObjectMap` and `Core\ObjectSet` are faster. A lookup
  with a `string`, `int` or object key is one hash lookup, as in an array. A `foreach` over either
  does not copy it." Run the `pages` check once (`rule:core-api/reference-card`).
  - [x] the `faster` highlight entry, and `unseen` records for the three members whose tests moved
  - [x] the newcomer review of the 15 texts, each finding fixed and stamped
  - [x] `pages` green

### 6. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and this goal's own tests at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 5 are done, one session each. Every acceptance check is green, `pages` included. The
module docs (`identity_store.rs:1`, `objmap.rs:4`, `cursor.rs`) hold every standing decision and
the trade-offs. The sweep's nightly failures (run 44510687786) were fixed before stage 4
(`4c82a2409a`, `9478fd2f66`), and no nightly has run since.

Figures as ns/op, statements, allocations, twin. Counts decide; the clock was noisy.

| Record | Before | After |
|---|---|---|
| `ObjectMap::get` | 1620, 3, 0, 0.85x | 2371, 3, 0, 2.23x |
| `ObjectMap::has` | 1867, 3, 0, 1.27x | 634, 3, 0, 2.33x |
| `ObjectMap::set` | 17536, 4, 0, 0.90x | 1787, 4, 0, under the floor |
| `ObjectSet::has` | 1680, 6, 0, 1.46x | 1949, 6, 0, 1.29x |
| iteration bench | fails `constant` (bytes slope 1.00) | 23047, 27, 7 (672 bytes), 1.38x, flat |

Every count is equal or lower and every growth flat. `bun nv scaling --budget` is red on 48
benches outside this goal.

### Next

Stage 6, one session: the goal's close. The pack names whether it is the chain's full gate.

- At a close: `cargo build`, `cargo clippy --all-targets -- -D warnings`, and this goal's own
  checks by name (`looper check --name` for each acceptance id). At a full gate: `looper verify`
  in the background, then `bun nv verify --doc` and `bun nv ci --check`.
- `bun nv owners --closes object-collections-fast`, then move each decision only this file holds
  into a module doc (the Standing decisions are already in `identity_store.rs` and `cursor.rs`)
  before the file is deleted.
- No newcomer review is owed: `site review` is green.

Expect no red from this goal. `bun nv verify --doc` may flag the new intra-doc links in
`identity_store.rs:1` (`crate::registry::constructor_symbol`, `Keying::admits`) if a private item
is linked from a public doc.

### Backlog

- A `tainted string` or `secret string` key keeps the chain. Stage 1 found that neither qualifier
  has run-time state, so the `string` keying could take both, if a program shows the need.
- A `uint` key keeps the chain. The `int` keying could take one whose value fits an `i64`, if a
  program shows the need.
- From `cheaper-forms`, which took it from `strings-in-place` and `arrays-in-place`:
  `Core\Arr::overlayDeep` on a sole subject still copies each nested map it merges into. A nested
  map that is itself sole could be changed in place. No ladder shows it yet.
