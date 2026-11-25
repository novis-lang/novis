---
looper: 1
title: a class is told when a property's value really changed
summary: "Reached when a class that implements the new global interface PropertyChangeObserver has onPropertyChanged(string $name, mixed $value) called after every write that changed a property's value by ==, compiled, erased, promoted and element writes alike, a write that stores an equal value calls nothing, a class that does not implement it pays nothing, and no element write copies its array."
context: [docs/rules/classes/property-observer.md, docs/rules/classes/property-observer-pipeline.md, docs/rules/classes/property-observer-costs-nothing-when-unused.md, docs/rules/expressions/equality-semantics.md, crates/nvs-hir/src/interfaces.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-types/src/expr_table.rs]
stages:
  - id: 1
    title: "the interface exists, and a whole write that changed the value calls onPropertyChanged"
    status: open
    context: [crates/nvs-types/src/iter_lib.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-runtime/src/object.rs, crates/nvs-codegen/tests/late_init.rs]
  - id: 2
    title: "an element write and an element unset report a change by comparing the element, and copy no array"
    status: open
    context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/lower/stmt.rs]
  - id: 3
    title: "the rules, the reference, the examples, the feature proofs and the website say so"
    status: open
    context: [docs/examples/lang/classes/propertyobserver/about.md, docs/reference/lang/50-classes.md, website/src/data/changes.json]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: whole
    stage: 1
    name: "a whole write, a promoted constructor store, a hooked write and an erased write call onPropertyChanged only when the slot's value changed by ==, after onPropertySet, and a class that does not implement the interface emits no comparison"
    run: [cargo, test, --test, property_changes]
    tests: [a_write_of_a_different_value_calls_on_property_changed, a_write_of_an_equal_value_calls_nothing, the_first_write_is_always_a_change, a_promoted_constructor_store_is_a_change, a_set_hook_is_compared_by_what_it_stored, an_erased_write_reports_a_change_like_a_compiled_one, on_property_set_runs_first_and_its_throw_skips_the_change, an_object_is_compared_by_identity, a_class_without_the_interface_emits_no_comparison]
    timeout: 1800
  - id: elements
    stage: 2
    name: "an element write reports a change only when the element differs or its key is new, an unset only when the key existed, and an append to an observed array copies nothing"
    run: [cargo, test, --test, property_changes]
    tests: [an_element_write_of_an_equal_value_calls_nothing, an_element_write_with_a_new_key_is_a_change, an_unset_of_a_missing_key_calls_nothing, an_append_in_a_change_observed_class_copies_no_array]
    timeout: 1800
  - id: proofs
    stage: 3
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, property-changes]
    timeout: 1800
  - id: close
    stage: 4
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, property-changes]
    timeout: 900
---

# a class is told when a property's value really changed

## Why

A user asked for the old value in `onPropertySet`, so an observer can compare it with the new one. The
common use is a record that tracks whether it still matches the database.
`docs/examples/lang/classes/propertyobserver/03-a-record-that-knows-it-changed.nvs` marks its record
unsaved even when a write stored the value it already had.

Passing the old value was weighed and turned down with the user (§ *Declined*). On an element write
the old whole array would have to stay alive across the write, so every element write in an observed
class would copy its array. The user then chose a separate signal: a third method that runs only when
the value really changed, in an interface of its own.

## Standing decisions

**The interface.** A new global interface `PropertyChangeObserver`, beside `PropertyObserver` and
`PropertyGuard` (`crates/nvs-hir/src/interfaces.rs:51`, `:95`, `:101`), with one method:
`onPropertyChanged(string $name, mixed $value): void`. It is independent of `PropertyObserver`: a
class implements either, both or neither. Registered like the guard's pair
(`crates/nvs-types/src/iter_lib.rs:207`), and a class that implements it must give the method a body,
as `ObserverCalls` requires for the observer (`crates/nvs-types/src/expr_table.rs:375`). Resolved
like `observer_calls` (`crates/nvs-types/src/expr/members.rs:2350`): a compile-time question read off
the declaration, dispatched virtually with the resolved label as fallback.

**What "changed" means.** The property's value before the write and after it differ by the language's
own `==` (`rule:expressions/equality-semantics`, `rule:expressions/object-identity-equality`,
`rule:expressions/mixed-equality` for a `mixed` or union property, `crates/nvs-ir/src/ir.rs:3475`).
So an object is compared by identity, an array by value, and `NAN` is always a change. The comparison
is emitted with the property's declared type, so a typed property never takes the `mixed` path.

**Where it runs in the pipeline.** Last, after `onPropertySet`:

```
Write:  beforePropertySet → set hook or storage → onPropertySet → onPropertyChanged (only if changed)
```

`$value` is what `onPropertySet` gets: the committed value, and the whole array for an element write.
A throw from the guard, the hook or `onPropertySet` fails the write before this step, so
`onPropertyChanged` does not run. A throw from `onPropertyChanged` fails the access after the store,
as the observer's does (`rule:classes/property-observer-pipeline`).

**Per kind of write.**
- **Whole write, no hook.** The old slot value is already read so the store can release it
  (`crates/nvs-ir/src/lower/mod.rs:4516` for the write-back; the plain store reads it the same way).
  It is compared with the new value before that release.
- **The first write** (the slot was `Unset`: a constructor store, a promoted parameter at
  `crates/nvs-ir/src/lower/mod.rs:2220`, a `lateinit` property written outside `initProperty`) is
  always a change. The runtime knows the slot was empty, so no `null` is invented.
- **A `set` hook.** The backing slot is read and retained before the hook runs, read again after it,
  and the two are compared. A hook that stores nothing reports no change. Every hooked property has
  its slot (`rule:classes/property-hooks`), so no `get` hook is called.
- **An erased write** (`mixed` receiver, `Core\Reflect`) runs the same step on the object's class at
  run time: `crates/nvs-runtime/src/object.rs:7153` for the hooked path and `:7219` for the plain one,
  beside `tell_erased_write` (`:7245`). `desc.conforms_to_name("PropertyChangeObserver")`, spelled as
  a literal for the reason `:7217` gives.
- **An element write** (`$p->tags["a"] = $v`, `$p->tags[] = $v`, a compound assignment, an
  increment, `??=` that writes) compares the old *element* with the new one at the leaf, before its
  `ArraySet` (`crates/nvs-ir/src/lower/mod.rs:4234`). A key that did not exist is a change. The
  result is carried to `write_back_array` (`mod.rs:4394`), which emits the call after
  `onPropertySet` (`mod.rs:4524`). The array is never copied for this, and nothing keeps the old array
  alive. A hooked property cannot take an element write at all (E0478, `mod.rs:4385`).
- **An element `unset`** is a change only when the key existed (`crates/nvs-ir/src/lower/stmt.rs:2878`).
- **No change is possible**, so nothing runs: a `??=` whose target already has a value, a `static`
  property, a write inside the property's own hooks, and a write that initializes a `lateinit`
  property while `initProperty` runs. These are the observer's own boundaries.

**A change observer that writes its own class's properties** recurses as the observer does, with no
re-entry guard. A write that stores an equal value ends that recursion, so the example that sets a
`$unsaved` flag needs no name test.

**Cost.** A class that does not implement `PropertyChangeObserver` emits nothing: no comparison, no
branch, no call (`rule:classes/property-observer-costs-nothing-when-unused` grows to say so, and the
test `a_class_without_the_interface_emits_no_comparison` pins it by the emitted IR). In a class that
implements it, each write pays one comparison. An int, a float, a bool, an enum case and an object
take one instruction. A string or an array first compares the pointer and the length, and scans only
two different values of equal length; the caller already built that value. A hooked write pays one
retain and one release. An element write pays one element read. Memory: nothing per request, and
nothing kept after the write. A write that stores an equal value skips the call, so it does less work
than an observer call. The test `an_append_in_a_change_observed_class_copies_no_array` proves the
element path by an allocation count, never a clock.

**Rules.** A new fragment `docs/rules/classes/property-change-observer.md` holds the interface, what
"changed" means, the per-kind table above in short, the cost line and § *Declined*. The pipeline
fragment's diagram and its write paragraph gain the last step. `docs/ground-rules.md` gets its line
from `bun nv rules`.

**Declined, written into the new fragment's Declined list.**
- **`onPropertySet` gets the old value as a third parameter.** An element write would have to keep the
  old whole array alive, so every element write in an observed class would copy the array, and a loop
  of appends would become quadratic.
- **The observer reports the change itself, like the guard: new element, old element and key path,
  plus an `onPropertyUnset`.** No copy, but every observer loses the whole array it gets today and
  writes a third method.
- **`onPropertySet` gets a `bool $changed` parameter.** Every observer pays the comparison and adds
  the parameter, and most bodies start by returning when it is `false`.
- **`onPropertySet` runs only on a change.** An observer could then no longer see a write that stores
  an equal value. `onPropertySet` stays "every write", and the new method is "every change".
- **`onPropertyChanged` as a third method of `PropertyObserver`.** Every observer would write an empty
  method and pay the comparison. A class pays only for the contract it implements, as the guard rule
  already decided.

**Feature proofs.** One new feature for the interface. `bun nv proofs --owed --goal property-changes`
names it and every feature the work reached. The three examples: a record that is marked unsaved only
by a real change, an audit line per change that skips equal writes, and an element write and an unset
that report only real changes. The bench is a whole write of an equal value and of a different value
in a change-observed class, declaring 0 allocations. The attack is a change observer that writes its
own properties, and every write ends because an equal value reports nothing. The existing example
`docs/examples/lang/classes/propertyobserver/03-a-record-that-knows-it-changed.nvs` stays an
`onPropertySet` example; its about.md gets one sentence that points to the new interface.

**Website.** One `website/src/data/changes.json` entry of kind `added` for the new feature, written to
AGENTS.md § *Text an end user reads*. The reference chapter `docs/reference/lang/50-classes.md` gets
its section beside `PropertyObserver` (`:3040`).

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is written to `### Backlog` with what blocked it.

**Lanes.** It runs as a side goal: it depends on no chain goal. It shares
`crates/nvs-types/src/expr/members.rs` with `extend-blocks` and `crates/nvs-ir/src/lower/` with any
chain goal that lowers a property write, so its branch merges `main` in before it lands, as every side
loop does. A class's `PropertyChangeObserver` never sees a property an `extend` block added, as
`extend-blocks` already says for the observer and the guard.

## Stages

### 1. the interface exists, and a whole write that changed the value calls onPropertyChanged

- [ ] `crates/nvs-hir/src/interfaces.rs:51`: `PropertyChangeObserver` in the list and as a constant
  beside `PROPERTY_OBSERVER` (`:95`), with its `type_params` test (`:173`).
  `rule:classes/property-observer-pipeline`; stage 3 writes the new interface's own fragment.
- [ ] `crates/nvs-types/src/iter_lib.rs:207`: the interface and its one method are registered; the
  conformance check requires a body.
- [ ] `crates/nvs-types/src/expr/members.rs:2350` and `crates/nvs-types/src/expr_table.rs:378`: the
  resolved `onPropertyChanged` label is recorded beside `ObserverCalls`, `None` for a class that does
  not implement the interface, for the compiled access and the promoted store
  (`expr_table.rs:2624`).
- [ ] `crates/nvs-ir/src/lower/expr.rs:6089` and `crates/nvs-ir/src/lower/mod.rs:2262`: the compiled
  whole write, the hooked write and the promoted store compare and call per § *Per kind of write*.
- [ ] `crates/nvs-runtime/src/object.rs:7153` and `:7219`: the erased hooked and plain writes do the
  same.
- [ ] `crates/nvs-codegen/tests/property_changes.rs` (new, beside `late_init.rs`): the nine tests of
  the check `whole`, on neutral classes (`Contact`, `Order`). Then `cargo test --test
  property_changes` once.

### 2. an element write and an element unset report a change by comparing the element, and copy no array

- [ ] `crates/nvs-ir/src/lower/mod.rs:4234` and the element-write paths that push `silenced_gets`
  (`mod.rs:4585`): the leaf compares the old element with the new one before its `ArraySet`, and a
  missing key is a change.
- [ ] `crates/nvs-ir/src/lower/mod.rs:4394`: `write_back_array` takes the result and emits
  `onPropertyChanged` after `onPropertySet` (`:4524`) only when it is true.
- [ ] `crates/nvs-ir/src/lower/stmt.rs:1962`, `:2096` and `:2878`: the compound, increment and
  `unset` paths carry the same result; an `unset` of a missing key is no change.
- [ ] The four tests of the check `elements` in `crates/nvs-codegen/tests/property_changes.rs`, the
  last one counting allocations. Then `cargo test --test property_changes` once.

### 3. the rules, the reference, the examples, the feature proofs and the website say so

**Split:** the examples, the attack and the bench go to one writing subagent once the rule fragment
is written. Its brief names AGENTS.md § *Text an end user reads* and the three examples of § *Feature
proofs*.

- [ ] `docs/rules/classes/property-change-observer.md` (new) and
  `docs/rules/classes/property-observer-pipeline.md`: per § *Rules*, then `bun nv rules`.
- [ ] `docs/rules/classes/property-observer-costs-nothing-when-unused.md`: one sentence that a class
  without `PropertyChangeObserver` emits no comparison.
- [ ] `docs/reference/lang/50-classes.md:3040`: the new section.
- [ ] The feature proofs `bun nv proofs --owed --goal property-changes` names: about.md, three
  examples, the bench, the attack, a conformance case and the help text.
- [ ] `docs/examples/lang/classes/propertyobserver/about.md`: one sentence pointing to
  `PropertyChangeObserver`.
- [ ] `website/src/data/changes.json:1`: the `added` entry, then `bun nv site --check stale`,
  `changes` and `review` green.

### 4. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by an interactive session on 2026-11-25, from a user report. The design was settled with the
user: a separate interface, run only on a real change. Nothing of this goal is built. Every anchor
above was read on that day, against a tree where `extend-blocks` was running.

### Next

Stage 1 from its first item. The check `whole` is red until `crates/nvs-codegen/tests/property_changes.rs`
exists with its nine tests.

### Backlog
