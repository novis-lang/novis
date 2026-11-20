---
looper: 1
title: "a revoked object leaves every Core collection, and any other use of it throws"
summary: "Reached when a root class may implement Revocable, revoke() removes the object from every ObjectMap, ObjectSet and Heap holding it, a property access, method call or clone of a revoked object throws RevokedObjectError naming where it was revoked, the check is compiled only where the static type could be revocable, and the feature has its proofs and pages."
context: [docs/rules/classes/interface-default-methods.md, docs/rules/classes/no-destructors.md, docs/reference/core/ObjectMap.md]
stages:
  - id: 1
    title: the rules are written, and the checker knows Revocable, its two methods and its error
    status: done
    context: [crates/nvs-hir/src/interfaces.rs, crates/nvs-types/src/iter_lib.rs, crates/nvs-hir/src/hierarchy.rs, crates/nvs-types/src/conformance.rs, crates/nvs-hir/src/errors.rs, docs/rules/statements/static-is-a-member-modifier.md, docs/rules/core-api/no-methods-on-scalars-or-arrays.md]
  - id: 2
    title: an object carries its revoked flag in its count word and its revoke site in one hidden slot
    status: done
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-runtime/src/throwable.rs, crates/nvs-types/src/layout.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-codegen/tests/refcount_calls.rs, docs/rules/classes/revocable-objects.md]
  - id: 3
    title: every use of a revoked object throws, and the check is compiled only where the type could be revocable
    status: open
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/ir.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-runtime/src/dispatch.rs, crates/nvs-runtime/src/graph.rs, crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/reflect.rs, docs/rules/classes/revocable-objects.md]
  - id: 4
    title: ObjectMap, ObjectSet and Heap drop a revoked object at once, and keep their count exact
    status: open
    context: [crates/nvs-stdlib/src/objmap.rs, crates/nvs-stdlib/src/objset.rs, crates/nvs-stdlib/src/heap.rs, crates/nvs-stdlib/src/identity_store.rs, crates/nvs-runtime/src/revoke.rs, crates/nvs-runtime/src/array.rs, docs/rules/core-api/collections-drop-revoked-objects.md]
  - id: 5
    title: the editor shows the cards of Revocable and its two methods
    status: open
    context: [crates/nvs-lsp/src/hover.rs, tests/lsp/hover/a-core-class-name-answers-its-card.lspt, crates/nvs-types/src/iter_lib.rs, docs/reference/tools/40-editor.md]
  - id: 6
    title: the feature's proofs
    status: open
    context: [docs/reference/lang/50-classes.md, docs/examples/lang/classes/stringable/about.md, docs/examples/types/DeprecatedError/about.md, benches/members/README.md, tests/hostile/README.md, docs/agent/prose.md]
  - id: 7
    title: the website explains revocable objects and lists the change
    status: open
    context: [website/src/content/docs/syntax/classes/standard-interfaces.mdx, website/src/content/docs/syntax/errors/error-types.mdx, website/src/content/docs/guides/cookbook/cache-value.mdx, website/src/data/changes.json, docs/agent/prose.md]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: checker
    stage: 1
    name: "only a root class may implement Revocable, only a class may, its two methods cannot be declared again, an ObjectMap of a subclass is not one of its root, and static in a collection's type arguments is bound at the call"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/revocable-a-class-with-a-parent-cannot-implement-it.nvst, --case, tests/conformance/reject/revocable-only-a-class-can-implement-it.nvst, --case, tests/conformance/reject/revocable-revoke-and-isrevoked-cannot-be-declared-again.nvst, --case, tests/conformance/reject/revocable-an-object-map-of-a-subclass-is-not-a-map-of-its-root.nvst, --case, tests/conformance/class/revocable-static-in-a-collection-type-is-bound-at-the-call.nvst]
    timeout: 1800
    tests:
      - tests/conformance/reject/revocable-a-class-with-a-parent-cannot-implement-it.nvst
      - tests/conformance/reject/revocable-only-a-class-can-implement-it.nvst
      - tests/conformance/reject/revocable-revoke-and-isrevoked-cannot-be-declared-again.nvst
      - tests/conformance/reject/revocable-an-object-map-of-a-subclass-is-not-a-map-of-its-root.nvst
      - tests/conformance/class/revocable-static-in-a-collection-type-is-bound-at-the-call.nvst
  - id: runtime-flag
    stage: 2
    name: "the revoked flag is masked from every count read, a revoked object is freed with its last reference, the first revoke site is kept, and a class that is not revocable has no hidden slot"
    run: [cargo, test, --lib, --, the_revoked_flag_is_masked_from_every_count_read, a_revoked_object_is_freed_with_its_last_reference, a_second_revoke_keeps_the_first_site, a_class_that_is_not_revocable_has_no_hidden_slot]
    timeout: 1800
    tests:
      - the_revoked_flag_is_masked_from_every_count_read
      - a_revoked_object_is_freed_with_its_last_reference
      - a_second_revoke_keeps_the_first_site
      - a_class_that_is_not_revocable_has_no_hidden_slot
  - id: compiled-release
    stage: 2
    name: "compiled code frees a revoked object, and the inline release of an object that is not revoked has no added instruction"
    run: [cargo, test, --test, revocable, --, compiled_code_frees_a_revoked_object, the_inline_release_of_an_object_that_is_not_revoked_is_unchanged]
    timeout: 1800
    tests:
      - compiled_code_frees_a_revoked_object
      - the_inline_release_of_an_object_that_is_not_revoked_is_unchanged
  - id: uses-throw
    stage: 3
    name: "a property read or write, a method call, clone, Json::encode, a reflective read, a call through an interface and a use through mixed of a revoked object throw RevokedObjectError naming the class and the revoke line, and a subclass inherits the interface"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/revocable-a-property-read-throws.nvst, --case, tests/conformance/class/revocable-a-property-write-throws.nvst, --case, tests/conformance/class/revocable-a-method-call-throws.nvst, --case, tests/conformance/class/revocable-clone-throws.nvst, --case, tests/conformance/class/revocable-json-encode-throws.nvst, --case, tests/conformance/class/revocable-a-reflective-read-throws.nvst, --case, tests/conformance/class/revocable-a-call-through-an-interface-throws.nvst, --case, tests/conformance/class/revocable-a-use-through-mixed-throws.nvst, --case, tests/conformance/class/revocable-the-error-names-the-class-and-the-revoke-line.nvst, --case, tests/conformance/class/revocable-a-subclass-inherits-it-from-the-root.nvst]
    timeout: 1800
    tests:
      - tests/conformance/class/revocable-a-property-read-throws.nvst
      - tests/conformance/class/revocable-a-property-write-throws.nvst
      - tests/conformance/class/revocable-a-method-call-throws.nvst
      - tests/conformance/class/revocable-clone-throws.nvst
      - tests/conformance/class/revocable-json-encode-throws.nvst
      - tests/conformance/class/revocable-a-reflective-read-throws.nvst
      - tests/conformance/class/revocable-a-call-through-an-interface-throws.nvst
      - tests/conformance/class/revocable-a-use-through-mixed-throws.nvst
      - tests/conformance/class/revocable-the-error-names-the-class-and-the-revoke-line.nvst
      - tests/conformance/class/revocable-a-subclass-inherits-it-from-the-root.nvst
  - id: uses-pass
    stage: 3
    name: "isRevoked, a second revoke, passing, storing, an array, a property, a variable, equality, a type test and ::class of a revoked object do not throw"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/revocable-isrevoked-does-not-throw.nvst, --case, tests/conformance/class/revocable-a-second-revoke-does-nothing.nvst, --case, tests/conformance/class/revocable-passing-and-storing-do-not-throw.nvst, --case, tests/conformance/class/revocable-an-array-a-property-and-a-variable-keep-it.nvst, --case, tests/conformance/class/revocable-equality-is-identity-and-does-not-throw.nvst, --case, tests/conformance/class/revocable-a-type-test-does-not-throw.nvst, --case, tests/conformance/class/revocable-the-class-name-does-not-throw.nvst]
    timeout: 1800
    tests:
      - tests/conformance/class/revocable-isrevoked-does-not-throw.nvst
      - tests/conformance/class/revocable-a-second-revoke-does-nothing.nvst
      - tests/conformance/class/revocable-passing-and-storing-do-not-throw.nvst
      - tests/conformance/class/revocable-an-array-a-property-and-a-variable-keep-it.nvst
      - tests/conformance/class/revocable-equality-is-identity-and-does-not-throw.nvst
      - tests/conformance/class/revocable-a-type-test-does-not-throw.nvst
      - tests/conformance/class/revocable-the-class-name-does-not-throw.nvst
  - id: checks-placed
    stage: 3
    name: "an access through a class type that is not revocable compiles no check, one through a revocable class or an interface compiles one, and revoke and isRevoked compile none"
    run: [cargo, test, --test, revocable, --, a_class_that_is_not_revocable_compiles_no_check, a_revocable_class_access_compiles_one_check, an_interface_call_compiles_one_check, revoke_and_isrevoked_compile_no_check]
    timeout: 1800
    tests:
      - a_class_that_is_not_revocable_compiles_no_check
      - a_revocable_class_access_compiles_one_check
      - an_interface_call_compiles_one_check
      - revoke_and_isrevoked_compile_no_check
  - id: collections
    stage: 4
    name: "an ObjectMap drops an entry whose key or value is revoked, an ObjectSet drops a revoked member, a Heap skips one on pop and peek with an exact count, a running loop skips one, one object leaves every collection, and a removed entry is not touched"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/revocable-an-object-map-drops-an-entry-whose-key-is-revoked.nvst, --case, tests/conformance/core/revocable-an-object-map-drops-an-entry-whose-value-is-revoked.nvst, --case, tests/conformance/core/revocable-an-object-set-drops-a-revoked-member.nvst, --case, tests/conformance/core/revocable-a-heap-pop-and-peek-skip-a-revoked-entry.nvst, --case, tests/conformance/core/revocable-a-heap-count-is-exact-after-a-revoke.nvst, --case, tests/conformance/core/revocable-a-loop-skips-an-entry-revoked-during-the-loop.nvst, --case, tests/conformance/core/revocable-one-object-leaves-every-collection-that-holds-it.nvst, --case, tests/conformance/core/revocable-a-removed-or-replaced-entry-is-not-touched-by-a-revoke.nvst]
    timeout: 1800
    tests:
      - tests/conformance/core/revocable-an-object-map-drops-an-entry-whose-key-is-revoked.nvst
      - tests/conformance/core/revocable-an-object-map-drops-an-entry-whose-value-is-revoked.nvst
      - tests/conformance/core/revocable-an-object-set-drops-a-revoked-member.nvst
      - tests/conformance/core/revocable-a-heap-pop-and-peek-skip-a-revoked-entry.nvst
      - tests/conformance/core/revocable-a-heap-count-is-exact-after-a-revoke.nvst
      - tests/conformance/core/revocable-a-loop-skips-an-entry-revoked-during-the-loop.nvst
      - tests/conformance/core/revocable-one-object-leaves-every-collection-that-holds-it.nvst
      - tests/conformance/core/revocable-a-removed-or-replaced-entry-is-not-touched-by-a-revoke.nvst
  - id: collections-memory
    stage: 4
    name: "a freed collection clears its back-links, a cycle of a map and its member is reclaimed cleanly, the request's end walks no back-link, and a membership is charged to the request and returned"
    run: [cargo, test, --test, revocable, --, a_freed_collection_clears_its_back_links, a_cycle_of_a_map_and_its_member_is_reclaimed_cleanly, the_request_end_walks_no_back_link, a_membership_is_charged_to_the_request_and_returned]
    timeout: 1800
    tests:
      - a_freed_collection_clears_its_back_links
      - a_cycle_of_a_map_and_its_member_is_reclaimed_cleanly
      - the_request_end_walks_no_back_link
      - a_membership_is_charged_to_the_request_and_returned
  - id: editor
    stage: 5
    name: "a hover on Revocable, a revoke() call and an isRevoked() call shows its card, and the root-only error names the root class"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/hover/a-revocable-interface-name-answers-its-card.lspt, --case, tests/lsp/hover/a-revoke-call-answers-its-card.lspt, --case, tests/lsp/hover/an-isrevoked-call-answers-its-card.lspt, --case, tests/lsp/diagnostics/revocable-below-the-root-names-the-root-class.lspt]
    timeout: 1800
    tests:
      - tests/lsp/hover/a-revocable-interface-name-answers-its-card.lspt
      - tests/lsp/hover/a-revoke-call-answers-its-card.lspt
      - tests/lsp/hover/an-isrevoked-call-answers-its-card.lspt
      - tests/lsp/diagnostics/revocable-below-the-root-names-the-root-class.lspt
  - id: feature-proofs
    stage: 6
    name: "revocable objects owe no proof"
    run: [bun, nv, proofs, --id, "lang:classes/revocable-objects"]
    timeout: 900
  - id: error-proofs
    stage: 6
    name: "RevokedObjectError owes no proof"
    run: [bun, nv, proofs, --id, RevokedObjectError]
    timeout: 900
  - id: site-changes
    stage: 7
    name: "every added or changed feature of this goal has its changes.json entry"
    run: [bun, nv, site, --check, changes]
    timeout: 900
  - id: site-review
    stage: 8
    name: "every user page this goal changed carries its newcomer review stamp"
    run: [bun, nv, site, --check, review]
    timeout: 900
---

# a revoked object leaves every Core collection, and any other use of it throws

## Why

An object stored in an array stays there until somebody removes it by hand. Often the object stands
for a database row. When the row is deleted, the developer must find every array, cache and side
table that holds the object and clean each one. A missed one keeps working on a deleted row, and
nothing reports an error.

Weak references do not fix it. A weak reference clears only when no strong reference remains, so it
answers "is it still in memory?", never "is it still valid?". In Novis this is a correctness
problem, not a memory one: the request heap is dropped whole at the request's end.

Removing the object from every plain array automatically was declined. Arrays are copy-on-write
values, so a value would change with no assignment: a loop shrinks, an index shifts and `count()`
goes stale. Finding the holders would also need a back-pointer on every array store, or a heap walk
on every delete.

This goal adds an opt-in. A root class writes `implements Revocable`, and gets `revoke()` and
`isRevoked()`. After `revoke()`, the object leaves every `Core\ObjectMap`, `Core\ObjectSet` and
`Core\Heap` at once. Any other use of it, from a plain array, a property or a variable, throws
`RevokedObjectError` naming where it was revoked.

The shape it serves, with the neutral names every test and example of this goal uses:

```nvs
final class IdentityMap
{
    private static Core\ObjectMap<string, Storable> $objectsByKey;

    public static function remember(Storable $object): void
    {
        self::$objectsByKey->set("{$object::class}#{$object->id}", $object);
    }
}

abstract class Storable implements Revocable
{
    public function delete(): void
    {
        // delete the row, then:
        $this->revoke();
    }
}

final class Post extends Storable { }
```

`$post->delete()` removes the post from the identity map and from every other `Core` collection that
holds it. A `Post` kept in a plain array stays there, and the next read of `$post->title` throws.

## Standing decisions

**Gate (all stages).** This goal builds on the direct-key store and the shared-store cursor of
`object-collections-fast`, and its examples use the keyed `foreach` of `keyed-iteration`. Stage 1
starts once `docs/goals/object-collections-fast.md` and `docs/goals/keyed-iteration.md` do not exist.
Both goals move code in `crates/nvs-stdlib/src/objmap.rs`, `objset.rs` and `identity_store.rs`, so
every anchor in those files is re-found with `bun nv peek --locate <symbol>` before it is used.

### What the program sees (the person's calls, settled; never reopened)

1. **Opting in is `implements Revocable`.** Only a class with no parent class may implement it,
   and every subclass inherits it. An interface cannot extend it, and an enum cannot implement it.
   A class with a parent that writes `implements Revocable` is a compile error whose help says to
   implement it on the root class. So a variable typed with a class that is not revocable never holds
   a revoked object. An abstract root class may implement it.
2. **Two methods come with the interface,** as default methods a class cannot declare again
   (`rule:classes/interface-default-methods` gains this exception): `public function revoke(): void`,
   which cannot be undone and does nothing the second time, and `public function isRevoked(): bool`.
3. **Using a revoked object throws `RevokedObjectError`, a subclass of `LogicError`.** It throws on a
   property read or write, a method call, `clone`, `Core\Json::encode` and a `Core\Reflect` read, call
   or write. It does not throw on `isRevoked()`, a second `revoke()`, passing the object, storing it,
   `==` (identity, `rule:expressions/equality-semantics`), the type test `is` (`rule:types/type-test`;
   the person's word was `instanceof`, which Novis writes `is`) or `::class`.
4. **`Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap` never return or count a revoked object.** An
   `ObjectMap` entry goes when its key or its value is revoked. A running loop skips an entry revoked
   during the loop. `count()` is always exact. A `Heap` skips a revoked entry on `pop` and `peek`, and
   its count drops at the `revoke()`.
5. **Plain arrays, properties and variables keep the reference.** A use of it throws as in 3.
6. **Typing.** The collections stay invariant, because they hold shared objects:
   `ObjectMap<string, Post>` is not an `ObjectMap<string, Storable>`. `ObjectMap<string, static>` is
   bound at the call as `array<static>` is (`rule:statements/static-is-a-member-modifier`). Stage 1's
   first item confirms this works today. **Fallback:** if it does not, this goal adds it, at the same
   binding site `array<static>` uses.
7. **A static property's default.** Stage 1 confirms whether `private static
   Core\ObjectMap<string, Storable> $objectsByKey = new Core\ObjectMap<string, Storable>();` compiles.
   **Fallback:** if it does not, the examples create the map on first use with `??=`, and nothing in
   the language changes. The rule records which one holds.

### How the runtime does it (the authoring session's calls, settled)

8. **The revoked flag is the top bit of the object header's count word**
   (`crates/nvs-runtime/src/object.rs:3423` `ObjHeader`, field `refcount: Cell<usize>`). The header
   stays the same size. Every read that compares or reports an object's count masks the bit. The
   immortal count stays `usize::MAX`, and no revocable object is ever immortal.
   - **The inline release stays one compare (my call).** `crates/nvs-codegen/src/emit.rs:4766` tests
     "count above one" with one unsigned compare. That compare's bound becomes the flag bit, so a count
     with the flag set takes the helper call, which masks. An object that is not revoked runs exactly
     the instructions it runs today. A revoked object pays one helper call per release, and is rare.
9. **Where the check is compiled.** Only where the static type could be revocable: a revocable class,
   an interface, `mixed`, `object`, a type parameter whose bound does not rule it out, and a union or
   nullable type that contains any of these. Never through a class type that is not revocable, which
   decision 1 makes sound. The check is one load and one predicted branch on the header line that a
   method call already reads, with a cold call to the runtime's thrower.
   - **One IR instruction (my call):** `CheckRevoked { object, site }` before a `FieldGet`,
     `FieldSet`, `CallVirtual`, a direct call with a receiver and a `Clone` whose object passes the
     test above. Erased access through `mixed` is checked inside the runtime's erased entries.
   - **`$this` is checked too (my call).** Inside a revocable class, `$this` has a revocable type, and a
     method may call `revoke()` and then read a property. Dropping a check that a dominating check on
     the same object already made, with no call between, is a later goal's work.
   - `revoke()` and `isRevoked()` are intrinsics. A call of either compiles no check.
10. **Removal is eager.** Only revocable classes get one hidden 8-byte slot. It is the first slot of
    the root class (`crates/nvs-types/src/layout.rs:1002` puts inherited fields first), so it has the
    same offset in every class of the tree. It is null until the object first enters a collection, and
    then points to the object's **memberships**: one entry per (collection, kind, key), where kind is
    map key, map value, set member or heap entry, and a map value's entry keeps its key.
    - Adding a revocable object to an `ObjectMap`, `ObjectSet` or `Heap` writes one membership.
      `remove()`, `clear()` and a `set()` that replaces a value remove the old one's membership.
    - `revoke()` sets the flag, follows the memberships, and removes each entry in O(1).
      `NvsArray::unset` (`crates/nvs-runtime/src/array.rs:1879`) leaves a hole and compacts later. A
      `Heap` entry stays in place, and the heap's count drops by one.
    - **Then the slot holds the revoke site (my call).** The memberships are freed, and the slot holds
      the call-site constant of the `revoke()` call, with its low bit set to tell it from a pointer. The
      error reads the class name and that site from the object, so the message costs nothing until it
      is thrown. A second `revoke()` keeps the first site.
    - **A collection freed during the request** clears its members' memberships in the walk that
      already releases its entries (`crates/nvs-runtime/src/object.rs:4895` `dismantle`). The cycle
      collector (`object.rs:3827` `reclaim`) clears them before it frees any object of the cycle, so
      the order of freeing never matters.
    - **At the request's end nothing is walked** (`rule:classes/no-destructors`). The sweep
      (`object.rs:3718`) frees the heap whole, and the memberships go with it. `revoke()` is an
      explicit call, and no user code runs on release.
11. **Names.** The runtime has an internal Rust type `LiveList` (`object.rs:3470`), the list of a
    context's allocated objects. Nothing user-facing says "live", and no new name here uses that word.
    The new runtime module is `crates/nvs-runtime/src/revoke.rs`, and its list type is `Memberships`.
12. **Memory.** 8 bytes per revocable object, and about 16 to 24 bytes per (object, collection)
    membership. All of it is allocated on the request heap, so the accounting allocator
    (`crates/nvs-runtime/src/budget.rs:1967`) charges it to the request, and it is freed with the
    request. It is O(in-flight), never O(requests served). The rule states it.

13. **`may_be_revocable` is a set, built once (stage 1's call).** `RevocableTypes::of(graph)` holds
    every name a concrete revocable class reaches, through `extends` or `implements`, itself included.
    A class type, an interface or a `Core` contract needs the check when it is in the set. `mixed`,
    `object` and a type variable need it when the set is not empty. A program with no revocable class
    compiles no check. This needs no lookup of a declaration's kind, and it is exact: a class type
    outside the set has no revocable subclass, because only a root opts in.
14. **An enum that writes `implements Revocable` keeps its one error (stage 1's call).**
    `rule:enums/no-class-machinery` already refuses every `implements` on an enum, so
    `E_REVOCABLE_NOT_A_CLASS` is for an interface alone. One mistake gets one error.
15. **Decision 7's answer: the default does not compile (`E0472`).** A static property's default is a
    compile-time constant. The examples create the map on first use with `??=`, in a static property
    typed `?Core\ObjectMap<…>`, and the rule says so.
16. **The flag tells the revoke site from the memberships pointer (stage 2's call).** Decision 10's
    low bit cannot work: a site blob is a byte array in the unit's data section, at any address. The
    hidden slot is a `null` `Value` whose payload is zero, the `Memberships` box while the object is
    not revoked, or the site blob once it is. `crate::revoke`'s module doc and
    `rule:core-api/collections-drop-revoked-objects` say so.
17. **The hidden slot is a named field, and its name is the class's flag (stage 2's call).**
    `flatten_fields` gives a revocable root, and so each subclass, a first slot named
    `nvs_runtime::revoke::HIDDEN_SLOT_NAME` (`#revocable`, which no property can be). Codegen calls
    `set_revocable` for a class whose first slot has that name, so no `revocable` field is carried
    through `ClassLayout` and `nvs_ir::ir::Class`, and the slot and the flag cannot disagree.
    `ClassDesc::field_slot` and `field_name` never return the hidden slot, so reflection by a name
    a program passes cannot reach it. `clone` leaves it null, the graph copy writes it as null, and
    `ClassDesc::first_property_slot` and `is_hidden_slot` are what every other walker asks.
18. **A call by name has no revoke site (stage 2's call).** `revoke()` through `mixed` or
    `Core\Reflect` runs in `call_erased_unchecked`, which has no site blob, so it keeps a null site,
    and the error names the class alone: "This `Blog\Post` object was revoked. A revoked object
    cannot be used." A written call keeps its line. Passing a site through the erased entry would
    add a parameter to every erased call for one rare form.

### More choices the authoring session made (settled)

- **`Revocable` sits on the compiler's interface roster** (`crates/nvs-hir/src/interfaces.rs:45`)
  and is seeded in `crates/nvs-types/src/iter_lib.rs`, as `Stringable` is. Its two methods are run by
  the runtime, not by a `Core` class's registry row. It is global, not `Core\Revocable`: the comment
  at `interfaces.rs:42` keeps every compiler-owned interface out of a namespace, as `Stringable` and
  `Comparable` are, and one rule for all of them is simpler than one exception.
- **`RevokedObjectError` is a global class**, as `RecursionError` is, since the runtime throws it on a
  language operation. It goes beside `Core\DeprecatedError` in `crates/nvs-hir/src/errors.rs:120` and
  `ThrownClass` (`crates/nvs-runtime/src/throwable.rs:341`).
- **The three compile errors**, with their constants in `crates/nvs-diagnostics/src/code/types_e08.rs`
  and the next free numbers when stage 1 runs (`E0899` is taken by `route-parameters`):
  `E_REVOCABLE_BELOW_ROOT`, `E_REVOCABLE_NOT_A_CLASS` (an interface that extends it, an enum that
  implements it) and `E_REVOCABLE_METHOD_DECLARED` (a class that declares `revoke` or `isRevoked`).
  Each has its card. No quick fix: the root class is often in another file, and the help names it.
- **Forms decision 3 does not list.** Anything that reads the object's properties throws:
  `serialize()`, a graph copy into a task or a cache (`crates/nvs-runtime/src/graph.rs:311`), and a
  `Stringable` or `Comparable` call, which is a method call. `Core\Debug` prints the class name and
  that the object is revoked, without its properties, and does not throw, so a program can inspect it.
- **`clone` of an object that is not revoked** makes a copy that is not revoked and is in no
  collection: its hidden slot starts null.
- **The bench.** Its operation is one `revoke()` of an object held in an identity map, then one `set`
  and one `get`, declared `// bench: complexity constant`. A `.scale.nvs` sibling grows the map from
  3,125 to 100,000 entries, and a `.twin.nvs` sibling does the same work with a plain
  `array<string, Post>` and `unset`. A `foreach` is not in it: it is linear, and the bench of
  `object-collections-fast` already compares a map's loop with a plain array's.

### The text an end user reads (AGENTS.md § *Text an end user reads*; the wording to use)

- `RevokedObjectError`'s message: "This `Blog\Post` object was revoked in `src/Blog/Post.nvs` on
  line 42. A revoked object cannot be used." The class and the place are the real ones.
- `E_REVOCABLE_BELOW_ROOT`: "`Blog\Post` cannot implement `Revocable`, because its parent class
  `Blog\Storable` does not." Help: "Write `implements Revocable` on `Blog\Storable`, the class
  with no parent. `Blog\Post` then gets it too."
- `E_REVOCABLE_NOT_A_CLASS`: "Only a class can implement `Revocable`."
- `E_REVOCABLE_METHOD_DECLARED`: "`Blog\Post` cannot declare `revoke()`. `Revocable` provides
  this method, and a class cannot replace it."
- The cards. `Revocable`: "An object that can be revoked. After `revoke()`, the object is removed
  from every `Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap`, and any other use of it throws
  `RevokedObjectError`." `revoke()`: "Revokes this object and removes it from every `Core\ObjectMap`,
  `Core\ObjectSet` and `Core\Heap`. After that, a property read or a method call throws
  `RevokedObjectError`. A second call does nothing." `isRevoked()`: "Returns `true` if `revoke()` was
  called on this object."

### The rules

- `docs/rules/classes/revocable-objects.md`: decisions 1 to 3, 5, 8 and 9, the forms above, and
  12's first half. **Declined:** weak references (they answer "in memory", not "valid"); automatic
  removal from plain arrays (a value changes with no assignment, and finding holders costs a
  back-pointer per array store or a heap walk per delete); a `revoke` callable only from inside the
  class (a public `delete()` already lets any holder end the row, so it adds nothing but a rule);
  `implements Revocable` below the root (every class-typed access would need the check); a
  `Disposable` or scope-exit hook (`rule:classes/no-destructors` declines it); an `ObjectArray` class
  (the name promises `[]`, an array's `foreach` and `Core\Arr`, which it cannot have, and
  `ObjectMap<string, T>` already has the shape); `ArrayAccess`-style hooks
  (`rule:iteration/no-magic-collection-interfaces`); lazy removal with a check on every read (it makes
  `count()` O(n) after a revoke; eager removal keeps reads on large collections fast).
- `docs/rules/core-api/collections-drop-revoked-objects.md`: decisions 4, 10 and 12.
- Each is recorded in `data/rules/<topic>/<slug>.json` with `"status": "designed"` and listed in
  `data/rules/classes.json` and `data/rules/core-api.json`. `bun nv rules --render` writes the
  chapters and `docs/ground-rules.md`. Stage 6 flips both to `shipped`.
- `rule:core-api/no-methods-on-scalars-or-arrays` names the three structures at its line 12, and stays
  as it is: they stay three.

### Trade-offs (the rules record them)

- **Security and request isolation:** a use after delete goes from silent to a loud error.
- **Latency:** a read from a collection costs what a plain array's read costs. Adding a revocable
  object costs one membership. `revoke()` costs O(collections holding it). An access through an
  interface, `mixed` or a revocable class pays one predicted branch. Every other access pays nothing.
- **Simplicity:** a user learns one interface, two methods and one error. The runtime gains one flag
  bit and one hidden slot.
- **Memory:** as decision 12.

### Lanes

Stages 1 to 4 run in order. Stage 2 needs stage 1's interface, stage 3 needs stage 2's flag, and
stages 2, 3 and 4 all change `crates/nvs-runtime/src/object.rs`. Stage 5 touches only
`crates/nvs-lsp` and `tests/lsp`, and may run as a side goal once stage 1 is done. Stage 6 waits for
stages 4 and 5, because its pages quote the final behaviour. Stage 7 waits for stage 6.

### Limits: what stops this goal

One attempt per item, and at most two sessions per stage. An item still red after its attempt goes
into the handoff with its cause, and the stage goes on with its other items. A stage still open after
two sessions stops the loop with `BLOCKED` naming the item. The counts decide: a bench or a guard is
read from its counts, never from a clock. A check, a case's expected output or a bound is never
loosened to pass.

## Stages

### 1. the rules are written, and the checker knows Revocable, its two methods and its error

**Gate:** `docs/goals/object-collections-fast.md` and `docs/goals/keyed-iteration.md` do not exist.

**Does:** writes both rules, declares the interface and the error to the checker, and makes the three
compile errors. Nothing runs yet.

**Split:** once the checker code lands, one writing subagent writes the five `.nvst` cases of the
`checker` check, each with `// covers: lang:classes/revocable-objects`. The session runs the check
once.

- [x] First: `crates/nvs-types/src/expr/calls.rs:160` binds `static` in a parameter or return type at
  the call. Confirm that `Core\ObjectMap<string, static>` in a static method's return type is bound
  there (the collection's type arguments are made at `crates/nvs-types/src/core_lib.rs:975`). If not,
  bind it at the same site (decision 6's fallback). The case
  `revocable-static-in-a-collection-type-is-bound-at-the-call.nvst` pins it. **It was not bound**:
  `bind_static` (`crates/nvs-types/src/signatures.rs:2521`) now rewrites a generic class's type
  arguments, and `yields_the_called_class` (`crates/nvs-types/src/check.rs:1567`) takes a fresh
  `new Core\ObjectMap<string, static>()` with no arguments, as it takes `[]`.
- [x] Then: `crates/nvs-types/src/defaults.rs:359` `eval_property_default` decides what a static
  property's default may be. Confirm whether decision 7's `new Core\ObjectMap<...>()` default compiles,
  and write the answer into § *State*. Change nothing in the language.
- [x] `docs/rules/classes/revocable-objects.md:1` and
  `docs/rules/core-api/collections-drop-revoked-objects.md:1`: the two rules of § *The rules*, with
  their records and their places in the topic lists. `docs/rules/classes/interface-default-methods.md:1`
  gains one sentence: a default of `Revocable` cannot be declared again. `bun nv rules --render`,
  then `bun nv rules --check` is green.
- [x] `crates/nvs-hir/src/interfaces.rs:45`: `Revocable` joins `RESERVED`, with its constant
  and the sentence at `:42`. `crates/nvs-types/src/iter_lib.rs:81`: `seed` adds it with `revoke(): void`
  and `isRevoked(): bool`, each with `has_body` set and a flag that refuses a redeclaration.
  No flag was needed: `conformance.rs` `reject_revocable_redeclaration` asks the graph.
- [x] `crates/nvs-hir/src/hierarchy.rs:761` `resolve_supertype`: `E_REVOCABLE_BELOW_ROOT` for a class
  with a parent, and `E_REVOCABLE_NOT_A_CLASS` for an interface or an enum.
  `crates/nvs-types/src/conformance.rs:768` `check_method_overrides`: `E_REVOCABLE_METHOD_DECLARED`.
  The three codes and their cards go in `crates/nvs-diagnostics/src/code/types_e08.rs:1254`, with the
  text of § *The text an end user reads*. They are E1001 to E1003 in `types_e10.rs`: `E0899` closed
  the `E08xx` range. The checks are in `HierarchyResolver::resolve`, after every link is in the graph,
  so the help can name a root declared in a later file.
- [x] `crates/nvs-hir/src/errors.rs:120`: `("RevokedObjectError", Some("LogicError"))`, and its
  constructor in `crates/nvs-types/src/error_lib.rs:267` if the tree needs one there. It needs none.
- [x] `crates/nvs-types/src/revocable.rs:1`: `may_be_revocable(ty)`, the test of decision 9, built on
  `implements_interface` (`crates/nvs-hir/src/hierarchy.rs:871`). Stage 3 calls it from the IR.
  It is `RevocableTypes::of(graph).may_be_revocable(ty, interner)`: see decision 13.
- [x] The `checker` check, run once. Green.

### 2. an object carries its revoked flag in its count word and its revoke site in one hidden slot

**Does:** the runtime half of decisions 8 and 10, without collections: the flag, the masked reads,
the hidden slot, `revoke()`, `isRevoked()` and the thrower. Nothing calls the thrower yet.

- [x] `crates/nvs-runtime/src/revoke.rs:1`: the flag constant, `Memberships`, `revoke(object, site)`,
  `is_revoked(object)` and `nvs_throw_revoked(object, site)`, which throws `RevokedObjectError` with
  § *The text an end user reads*'s message. `ThrownClass` gains `Revoked`
  (`crates/nvs-runtime/src/throwable.rs:341`, its name at `:438`).
- [x] The masked count reads. The three readers mask: `fn refcount` (`object.rs:4029`),
  `NvsObj::refcount` (`:4510`) and `NvsObj::refcount_of` (`:4603`). The raw readers mask too:
  `bump` (`:4738`), `drop_one` (`:4782`) and the `Debug` field (`:4659`). The sweep's tally
  (`:3881`, `:3890`), `unwind_abandoned` (`:4961`), `value.rs:707`, `graph.rs:584`,
  `crates/nvs-stdlib/src/instance.rs:1796` and `crates/nvs-stdlib/src/identity_store.rs:201` read
  through those readers, and each is confirmed to.
- [x] `crates/nvs-codegen/src/emit.rs:4766`: the inline release's "above one" bound becomes the flag
  bit, as decision 8 says. The asserts at `emit.rs:112` gain the flag's. `emit.rs:4795`'s retain needs
  no change.
- [x] `crates/nvs-types/src/layout.rs:1002` `flatten_fields`: a revocable root class gets the hidden
  slot first, and the runtime's class descriptor carries it. A class that is not revocable is
  unchanged.
  - [x] List every walker of a descriptor's field names or slots (reflection, `Core\Json`, `Core\Debug`,
    `serialize`, the graph copy, `clone`, the codecs, the constructor's defaults) with an `Explore`
    subagent.
  - [x] `flatten_fields` puts the hidden slot first for a class whose root implements `Revocable`,
    and codegen calls `ClassTable::set_revocable` for a class whose first slot is the hidden one
    (decision 17: no `revocable` field on `ClassLayout` or `nvs_ir::ir::Class`).
  - [x] Every walker skips slot 0 of a revocable descriptor; `clone` and the graph copy start it null.
  - [x] `a_class_that_is_not_revocable_has_no_hidden_slot` in `layout.rs`'s tests.
- [x] `revoke()` and `isRevoked()` lower to calls of `revoke` and `is_revoked` with the call site's
  constant. A reflective or erased call of either reaches the same functions.
  - [x] `InstKind::Revoke` and `InstKind::IsRevoked` in `crates/nvs-ir/src/ir.rs`, lowered in
    `lower_method_call` for a call resolved to `Revocable`, which releases the transferred receiver.
  - [x] Codegen calls `nvs_object_revoke` and `nvs_object_is_revoked` with their own signatures,
    and the three externs are in the runtime's symbol table.
  - [x] `call_erased_unchecked` (`crates/nvs-runtime/src/dispatch.rs`) runs both by name, so a call
    through `mixed` and a `Core\Reflect` call reach them (decision 18).
- [x] The four tests of the `runtime-flag` check in `revoke.rs`'s test module, and
  `crates/nvs-codegen/tests/revocable.rs:1` with the two tests of the `compiled-release` check, which
  read the Cranelift text as `crates/nvs-codegen/tests/refcount_calls.rs` does. Each Rust test file
  carries `// covers: lang:classes/revocable-objects`. Both checks run once.

### 3. every use of a revoked object throws, and the check is compiled only where the type could be revocable

**Does:** decisions 3 and 9: the IR instruction, its code, the erased entries and the library entries
that read an object.

**Split:** once the code lands, one writing subagent writes the seventeen `.nvst` cases of the
`uses-throw` and `uses-pass` checks under `tests/conformance/class/`, each with `// covers:
lang:classes/revocable-objects`, and the throwing ones with `RevokedObjectError` too. The session
runs both checks once.

- [ ] `crates/nvs-ir/src/ir.rs:1291`: `InstKind::CheckRevoked { object, site }`.
  `crates/nvs-codegen/src/emit.rs:1029`: its code, a load of the count word, a test of the flag and a
  cold call to `nvs_throw_revoked`.
- [ ] `crates/nvs-ir/src/lower/expr.rs`: the instruction before each site whose object
  `may_be_revocable` accepts: the property read (`lower_property_access`, `:5963`, `FieldGet` at
  `:6125`), the property write (near `:6435`), the method call (`lower_method_call`, `:4782`,
  `CallVirtual` at `:5074`, and a direct call with a receiver) and `clone` (`lower_clone_expr`,
  `:7891`).
- [ ] The erased entries test the flag: `call_erased_method_from` (`crates/nvs-runtime/src/dispatch.rs:408`),
  `read_erased_property_owned` (`crates/nvs-runtime/src/object.rs:6117`) and
  `write_erased_property` (`object.rs:6526`). A call of `revoke` or `isRevoked` passes.
- [ ] The library reads: `Core\Json::encode`'s object walk (`crates/nvs-stdlib/src/json.rs:1859`),
  `Core\Reflect`'s `get`, `set` and `call` (`crates/nvs-stdlib/src/reflect.rs:2967`, `:3040`,
  `:3122`, through the erased entries above), the graph copy (`crates/nvs-runtime/src/graph.rs:311`)
  and `nvs_object_clone` (`object.rs:5105`, for a clone through `mixed`). `Core\Debug` prints the class
  and "revoked" without throwing.
- [ ] `crates/nvs-codegen/tests/revocable.rs`: the four tests of the `checks-placed` check, run once.
- [ ] The seventeen cases, and the `uses-throw` and `uses-pass` checks run once.

### 4. ObjectMap, ObjectSet and Heap drop a revoked object at once, and keep their count exact

**Does:** decisions 4, 10 and 12 for the three collections.

**Split:** once the code lands, one writing subagent writes the eight `.nvst` cases of the
`collections` check under `tests/conformance/core/`, each with `// covers:
lang:classes/revocable-objects`. The session runs the check once.

- [ ] `crates/nvs-runtime/src/revoke.rs`: `link(object, collection, kind, key)` and `unlink`, and
  `revoke`'s walk, which removes a map entry or a set member with `NvsArray::unset`
  (`crates/nvs-runtime/src/array.rs:1879`) and drops a heap's count by one.
- [ ] `Core\ObjectMap` (`crates/nvs-stdlib/src/objmap.rs`): `set` (`:365`) links the key and the
  value and unlinks a replaced value, `remove` (`:457`) and `clear` (`:535`) unlink, and `count`
  (`:474`) stays the store's count. A loop over the shared store (`iterate`, `:523`) skips a hole.
- [ ] `Core\ObjectSet` (`crates/nvs-stdlib/src/objset.rs`): `add` (`:375`) links, `remove` (`:413`) and
  `clear` (`:515`) unlink. `union`, `intersect` and `diff` link each member of the set they make.
- [ ] `Core\Heap` (`crates/nvs-stdlib/src/heap.rs`): `push` (`:655`) links. `pop` (`:693`) and `peek`
  (`:679`) skip a revoked entry and unlink it. `count` (`:733`) is the heap's own count, which
  `revoke` lowers. `iterate` (`:759`) skips a revoked entry.
- [ ] Release: `dismantle` (`crates/nvs-runtime/src/object.rs:4895`) unlinks each member of one of
  the three collections as it releases its entries. `reclaim` (`object.rs:3827`) unlinks within a
  cycle before it frees. The sweep (`object.rs:3718`) walks no membership.
- [ ] `crates/nvs-codegen/tests/revocable.rs`: the four tests of the `collections-memory` check. The
  request-end test counts membership walks with a counter beside
  `nvs_runtime::budget::refcount_calls`, and the memory test reads the request's charge
  (`crates/nvs-runtime/src/budget.rs:1174`). Run once.
- [ ] The eight cases, and the `collections` check run once.

### 5. the editor shows the cards of Revocable and its two methods

**Does:** a hover on the interface's name and on a call of either method shows its card from
§ *The text an end user reads*. The root-only error reaches the editor as every diagnostic does.

**Split:** once the code lands, one writing subagent writes the four `.lspt` cases of the `editor`
check, copying the shape of `tests/lsp/hover/a-core-class-name-answers-its-card.lspt` and
`docs/agent/conventions.md` § *An `.lspt` case*. The session runs the check once.

- [ ] `crates/nvs-types/src/iter_lib.rs:81`: the three cards sit beside the seeded interface, as
  `rule:core-api/reference-card` keeps a card beside its declaration.
- [ ] `crates/nvs-lsp/src/hover.rs:891` `core`: a hover on `Revocable`, `revoke()` or
  `isRevoked()` returns its card.
- [ ] `docs/reference/tools/40-editor.md:1`: no change unless a section lists the cards a hover shows.
- [ ] The four `.lspt` cases, and the `editor` check run once.

### 6. the feature's proofs

**Does:** `lang:classes/revocable-objects` and `RevokedObjectError` get every proof
`rule:testing/feature-proofs` lists. Every file here is text an end user reads, and every `.nvs`
comment passes `bun nv proofs --comments`.

**Split:** three writing subagents, whose files are disjoint, run side by side once the session has
written the reference section: one writes the three examples and their `.out` files, one the two
attacks, one the bench and its two siblings. Each is told the reference section's path, the error
message and the neutral names of § *Why*. The session writes the reference section and the
`about.md` and `details.md` files, and runs both proof checks once.

- [ ] First: `bun nv proofs --id lang:classes/revocable-objects` and `bun nv proofs --id
  RevokedObjectError` print what each owes. If the roster gives `revoke` and `isRevoked` ids of their
  own, each id gets its own check in this goal's front matter and its proofs here.
- [ ] `docs/reference/lang/50-classes.md:3902`: a `#` section *Revocable objects* after `Parses` and
  before *Objects are handles*. It makes the feature and its help (`nvs agent show`). It says what the
  interface does, the two methods, what throws and what does not, and the three collections.
- [ ] `docs/examples/lang/classes/revocable-objects/about.md:1` and `details.md:1`.
- [ ] Three examples in the same folder, each with its `.out`: `01-an-identity-map-forgets-a-deleted-row`
  (the identity map of § *Why*, walked with a keyed `foreach`), `02-a-set-of-online-users` (a user who
  logs out is revoked and leaves the set), `03-a-job-heap-skips-a-cancelled-job` (a cancelled job is
  revoked, and `pop` returns the next one).
- [ ] `tests/hostile/lang/classes/revocable-objects/01-keep-a-revoked-object-and-use-it.nvs:1`:
  `// Attack:` a request keeps a revoked object in a plain array, a static property and a closure.
  Each numbered step tries one use: a read, a write, a call, a clone, `Core\Json::encode` and a
  `Core\Reflect` read. Every step should throw `RevokedObjectError`, and the runtime should stay standing.
- [ ] `benches/members/lang/classes/revocable-objects.nvs:1`, with `.scale.nvs` and `.twin.nvs`, as
  § *More choices* describes. `// bench: complexity constant`.
- [ ] `docs/examples/types/RevokedObjectError/about.md:1`, one example
  `01-catching-a-use-after-delete.nvs` with its `.out`, and the attack
  `tests/hostile/types/RevokedObjectError/01-revoke-inside-a-catch-and-a-finally.nvs:1`, as
  `Core\DeprecatedError`'s are laid out.
- [ ] Both rules flip to `"status": "shipped"`, with their `guardedBy` lists naming the cases of the
  checks above.
- [ ] The `feature-proofs` and `error-proofs` checks run once, and every proof they name is written.

### 7. the website explains revocable objects and lists the change

**Gate:** `docs/goals/website-follows-changes.md` does not exist.

**Does:** every page a user reads says what changed. Every page follows AGENTS.md § *Text an end
user reads*.

- [ ] `website/src/content/docs/syntax/classes/revocable-objects.mdx:1`: the page *Revocable
  objects*, with `covers: ['lang:classes/revocable-objects', 'RevokedObjectError']` and a
  `FeatureSection` of the examples folder. Its sidebar entry goes after *Stringable, Comparable and
  Parses* in `website/astro.config.ts:705`.
- [ ] `website/src/content/docs/syntax/errors/error-types.mdx:28`: a row for `RevokedObjectError`
  (`LogicError`, "a use of an object after `revoke()` was called on it"), and the name in `covers`.
- [ ] `docs/reference/core/ObjectMap.md:1`, `ObjectSet.md:1` and `Heap.md:1`: each introduction says in
  one or two sentences that a revoked object leaves the collection, and links the new page. The map's
  example keys an identity map by `"{$className}#{$id}"`.
- [ ] `website/src/content/docs/guides/cookbook/remove-deleted-row.mdx:1`: the recipe *Remove a
  deleted row everywhere*, built on the identity map, with its snippet
  `website/snippets/guides/cookbook/remove-deleted-row/remove-deleted-row.nvs` and `.out`. Its sidebar
  entry is beside *Cache a slow result* (`website/astro.config.ts:510`), and its line goes in
  `website/src/content/docs/guides/cookbook/index.mdx:21`.
- [ ] `website/src/data/changes.json:1`: one `new` entry for revocable objects
  and `RevokedObjectError`, in the shape `website-follows-changes` set.
- [ ] `bun nv site --stale` lists the pages this goal made stale. Each is read, corrected and stamped
  with `bun nv site --stamp`.
- [ ] The `site-changes` check run once.

### 8. the goal is verified once, and every failure is fixed

- [ ] The newcomer review `website-follows-changes` requires, for every user page this goal changed,
  with its stamp. The `site-review` check run once.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and this goal's own tests at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 and 2 are built, and both stage 2 checks are green (`runtime-flag`, `compiled-release`, run
once each).
- `67319e03c0`: `flatten_fields` gives a revocable root, and each subclass, the hidden slot first,
  named `nvs_runtime::revoke::HIDDEN_SLOT_NAME`. Codegen calls `set_revocable` for a class whose first
  slot has that name (decision 17). `ClassDesc::field_slot` and `field_name` never return it,
  `first_property_slot` and `is_hidden_slot` are new, `clone` leaves it null, the graph copy writes it
  as null, and `Core\Json`'s shape cursor and the test library's object diff skip it.
- The lowering slice: `InstKind::Revoke` and `InstKind::IsRevoked` (`crates/nvs-ir/src/ir.rs`), lowered
  in `lower_method_call` for a call resolved to `Revocable`; their code in `emit.rs` with the
  `revoke` and `is_revoked` signatures; the three externs in `helpers.rs`'s symbol table; and
  `call_erased_unchecked` runs both by name with a null site (decision 18).
  `compiled_code_frees_a_revoked_object` is a live-bytes slope test.

Nightly run 44510687786 ran at `adc7487`, before its fixes (`eb5c02e983`, `4c82a2409a`, `656f5b2f2d`,
`1c75c4ef0e`), which cover every job it named; no nightly has run since.

### Next

Stage 3, from its first item: `InstKind::CheckRevoked { object, site }` beside `Revoke` in
`crates/nvs-ir/src/ir.rs` (its rows in `for_each_operand`, `effects` and `print.rs` beside `Revoke`'s),
and its code in `emit.rs` beside the `Revoke` arm: a load of the count word at
`nvs_runtime::OBJ_REFCOUNT_OFFSET`, a test of `REVOKED` that is not the immortal count, and a cold call
of `nvs_throw_revoked` (already in the symbol table) with a status check. The anchors stage 3's items
give in `expr.rs` moved; find each with `bun nv peek --locate lower_property_access lower_method_call
lower_clone_expr`. The method call's check goes after the `Revocable` intrinsic return in
`lower_method_call`, so `revoke()` and `isRevoked()` compile none. The erased entries' flag test must
come after the intrinsic block in `call_erased_unchecked` (`crates/nvs-runtime/src/dispatch.rs`), so a
call of either by name passes. Then the stage's split: one writing subagent for the seventeen `.nvst`
cases.

No existing test is expected to go red. A test that lists every `InstKind`, or every symbol in
`helpers.rs`'s table, would be the one to look at first if one does.

### Backlog
