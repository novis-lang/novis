---
looper: 1
title: "Core\\Reflect\\ClassInfo::parent names the class a class extends"
summary: "Reached when Core\\Reflect\\ClassInfo::parent returns the full name of the class a described class extends, or null for a class that extends none and for an interface, a program walks the chain and reads each class's attributes, and the rule, the reference card and the feature proofs say so."
context: [crates/nvs-stdlib/src/reflect.rs, crates/nvs-runtime/src/object.rs, docs/rules/core-classes/reflect.md]
stages:
  - id: 1
    title: the descriptor carries the parent class, and ClassInfo::parent returns it
    status: done
    context: [crates/nvs-types/src/layout.rs, crates/nvs-ir/src/ir.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-codegen/src/lib.rs]
  - id: 2
    title: the reference card, the examples, the attack and the bench say what parent does
    status: done
    context: [docs/examples/core/Reflect-ClassInfo/attributes/about.md, docs/agent/prose.md]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: case
    stage: 1
    name: "parent names the extended class at every level, is null for a root class and an interface, and a walk up the chain reads each level's own attributes"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/reflect-parent-walks-the-chain-and-reads-each-levels-attributes.nvst]
    timeout: 1800
  - id: rust
    stage: 1
    name: "a descriptor's parent is the class it extends, and parent returns null for a root class, an interface and a descriptor with none set"
    run: [cargo, test, --lib, --, class_info_parent_is_the_extended_class_and_null_at_a_root]
    timeout: 1800
    tests:
      - class_info_parent_is_the_extended_class_and_null_at_a_root
  - id: proofs
    stage: 2
    name: "Core\\Reflect\\ClassInfo::parent has every feature proof"
    run: [bun, nv, proofs, --id, "Core\\Reflect\\ClassInfo::parent"]
    timeout: 1800
---

# Core\Reflect\ClassInfo::parent names the class a class extends

## Why

A framework reads a class's attributes at run time with `Core\Reflect\ClassInfo::attributes()`, for a
class whose name it only learns at run time. An abstract base class often declares properties, with
attributes, that every subclass shares. `ClassInfo::properties()` lists the base class's properties,
but `attributes()` lists only what the described class's own declaration carries, and no member of
`ClassInfo` names the class it extends. So the framework has to name each base class itself.

A user asked for either `ClassInfo::parent(): ?string` or an `attributes()` that includes the parents'
attributes. The person chose `parent()`.

## Standing decisions

**The member (the person's call).** `Core\Reflect\ClassInfo::parent(): ?string` returns the full name of
the class the described class extends, as `name()` writes a name, or `null`. A program walks the chain
with `Core\Reflect::forClass($info->parent())` and reads each level's own `attributes()`.

**`attributes()` stays own-only (the person's call).** It is not changed to include the parents'
attributes. The Declined list in `rule:core-classes/reflect` says why: it reverses the decided
own-only reading (`crates/nvs-stdlib/src/reflect.rs:157`), existing callers would get more rows than
before, and the runtime would have to invent an order for an attribute written at two levels. A program
that wants the merge writes the loop, and decides the order itself.

**`?string`, not `?ClassInfo` (my call).** A name costs one string per call. A description builds one
row object per property, method and constant, which a caller that only wants the name pays for nothing.
It matches `name()`, and `forClass` turns the name into a description when the caller wants one.

**An interface has no parent (my call).** An interface's `extends` list names interfaces, possibly
several, and none of them is a parent class. `parent()` returns `null` for an interface. The interfaces
a class implements are not reached by this goal.

**A `Core` parent is named (my call).** A program class may extend a `Core` class. `parent()` returns
that name, because it is true. `forClass` already returns `null` for a `Core` class
(`rule:core-classes/reflect`), so a walk stops there with no special case.

**Where the name comes from (my call).** `nvs_types::layout::ClassLayout` gains `parent: Option<String>`,
the class's own `extends` entry, rendered as `conforms` renders its names
(`crates/nvs-types/src/layout.rs:136`). `nvs_ir::ir::Class` carries it, codegen stores it on the
descriptor with a setter beside `set_class_attributes`, and `ClassDesc` holds `parent: Option<String>`,
`None` by default. Every other place that builds an IR `Class` sets `parent: None`. The layout tells
an interface from a class by a set `collect_own` fills, because `nvs_hir::ClassLinks` does not say
which kind a declaration is.

**`parent()` reads the descriptor at call time (my call).** It finds the descriptor by the
description's name through `Ctx::class_desc`, as `construct` does, rather than reading a slot
`describe` fills. A description that never asks pays nothing, and a name the table does not reach
returns `null`.

**What it costs.** One `String` per class that extends one, once per process and never per request.
Per `parent()` call, one string, charged to the request that asked. Nothing on any other path: no
existing descriptor read changes.

**Where it is written down.** `docs/rules/core-classes/reflect.md` gains a paragraph on `parent()` and a
Declined list. The module doc's own-only decision in `crates/nvs-stdlib/src/reflect.rs:157` and the
`attributes` reference card (`crates/nvs-stdlib/src/reflect.rs:1721`) point to `parent()` for a walk.

**Lanes.** Stage 1 changes the compiler, the runtime and `crates/nvs-stdlib/`. Stage 2 changes
`docs/examples/`, `tests/hostile/` and `benches/members/` alone, and needs stage 1's member.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. the descriptor carries the parent class, and ClassInfo::parent returns it

**Does:** the keystone. The class a class extends reaches its runtime descriptor, and `ClassInfo`
returns it.

- [x] `crates/nvs-types/src/layout.rs:542`: `ClassLayout` gains `parent: Option<String>` beside
  `conforms` (`:136`), set from the class's own `extends` entry in the class graph that
  `collect_conforms` (`:1125`) reads. It is `None` for an interface and for a class that extends none.
- [x] `crates/nvs-ir/src/ir.rs:477`: `Class` gains `parent: Option<String>` beside `conforms`.
  `crates/nvs-ir/src/lower/mod.rs:1316` copies the layout's. `crates/nvs-ir/src/lower/mod.rs:1382`,
  `crates/nvs-ir/src/lower/anon_fn.rs:371`, `crates/nvs-ir/src/lower/anon_fn.rs:1250` and
  `crates/nvs-ir/src/lower/generator.rs:565` set `None`.
- [x] `crates/nvs-runtime/src/object.rs:648`: `ClassDesc` gains `parent: Option<String>` with its cost
  in its doc comment, a `ClassTable::set_parent` beside `set_class_attributes` (`:2983`), and a
  `ClassDesc::parent()` reader. `crates/nvs-codegen/src/lib.rs:2215` calls the setter when the IR class
  has one.
- [x] `crates/nvs-stdlib/src/reflect.rs:638`: the `parent` instance method beside `attributes`, returning
  `?string`, with its reference card held to `AGENTS.md` § *Text an end user reads*. The card says it
  returns `null` for a class that extends none and for an interface, and that `forClass` returns `null`
  for a `Core` parent. The Rust test `class_info_parent_is_the_extended_class_and_null_at_a_root`
  beside `class_info_attributes_is_one_row_per_attach_site_in_the_descriptors_order` (`:4591`). The
  `rust` check runs once.
- [x] `crates/nvs-stdlib/src/reflect.rs:157` and the `attributes` card (`:1721`): the own-only text
  points to `parent()` for a walk up the chain. `docs/rules/core-classes/reflect.md:1` gains the
  paragraph and the Declined list from § *Standing decisions*. Then `bun nv rules --render`, and
  `bun nv rules --check` is green.
- [x] `tests/conformance/core/reflect-parent-walks-the-chain-and-reads-each-levels-attributes.nvst:1`:
  an abstract base class with attributes on its properties, a middle class and a leaf. `parent()` from
  the leaf names each level and is `null` at the root and on an interface. A loop up the chain prints
  each level's own `attributes()`. The `case` check runs once.

### 2. the reference card, the examples, the attack and the bench say what parent does

**Does:** the feature proofs. Every line here is text an end user reads. Run
`bun nv proofs --id 'Core\Reflect\ClassInfo::parent'` first and last.

**Split:** one writing subagent writes the attack and the bench while the session writes `about.md` and
the examples. Their files are disjoint, and no one builds until both end.

- [x] `docs/examples/core/Reflect-ClassInfo/parent/about.md:1`: what `parent` returns, and the walk that
  reads a base class's attributes.
- [x] `docs/examples/core/Reflect-ClassInfo/parent/01-print-the-parent-of-a-class.nvs:1`, `02-walk-up-to-the-root.nvs`
  and `03-read-the-attributes-of-every-base-class.nvs`, each with its `.out`.
- [x] `tests/hostile/core/Reflect-ClassInfo/parent/01-a-walk-that-tries-to-reach-a-private-member-of-a-parent.nvs:1`:
  a walk up the chain reaches the parent's description and tries to read its `private` property. The
  read fails as it does without reflection (`rule:security/reflection-enforces-visibility`).
- [x] `benches/members/core/Reflect-ClassInfo/parent.nvs:1`, modelled on
  `benches/members/core/Reflect-ClassInfo/name.nvs`.
- [x] The `proofs` check is green, and `bun nv proofs --comments` passes on the new files.

### 3. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and this goal's own tests at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done. `bun nv proofs --id 'Core\Reflect\ClassInfo::parent'` prints `complete.`,
and `bun nv proofs --comments` passes on the five new files. The examples are under
`docs/examples/core/Reflect-ClassInfo/parent/`, the attack is
`tests/hostile/core/Reflect-ClassInfo/parent/01-a-walk-that-tries-to-reach-a-private-member-of-a-parent.nvs`
(six steps, exit 0, no `// hostile:` line), and the bench
`benches/members/core/Reflect-ClassInfo/parent.nvs` declares `calls 1` and `allocations 1`: one
string per `parent()` call. All 40 `Core\Reflect` perf records were measured again on a release
binary built after the last edit to `crates/nvs-stdlib/src/reflect.rs`, so do not edit that file
again, doc comments included, or they go stale.

The sweep's `env-secret-proofs` red is stale: `git log -S` names cheaper-forms' retire as its last
hit, and no check by that name exists in the tree.

### Next

Stage 3 closes the goal with a build, clippy and this goal's own checks (`case`, `rust`, `proofs`),
unless the pack names this goal as the full gate. 147 perf records outside `Core\Reflect`
(`Core\Str`, `Core\Json`, `Core\IO` and more) are stale from earlier goals' edits, and the full
gate's `bun nv ci --check` will name them. `website/public/llms-full.txt` lists the `ClassInfo`
members and may go red until it is regenerated. `cargo fmt --check` names 18 places from earlier
goals.

### Backlog
