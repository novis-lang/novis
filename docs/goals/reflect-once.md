---
looper: 1
title: "Core\\Reflect describes each class once per process, and PropertyInfo tells lateinit and readonly"
summary: "Reached when every class's Core\\Reflect description is built once when the unit loads as immortal objects shared by every core, forClass, forObject and EnumInfo::of allocate nothing, every roster array is built at most once per request, PropertyInfo answers isLateInit and isReadonly, nvs meta --json writes both modifiers in a property signature, and the rules, the reference, the examples and the feature proofs say so."
context: [crates/nvs-stdlib/src/reflect.rs, docs/rules/core-classes/reflect.md]
stages:
  - id: 1
    title: "PropertyInfo answers isLateInit and isReadonly, and nvs meta writes both modifiers"
    status: open
    context: [crates/nvs-cli/src/meta.rs, docs/rules/classes/lateinit.md, docs/rules/tooling/meta-json-takes-a-program.md]
  - id: 2
    title: "every description is built once when the unit loads, as immortal objects"
    status: open
    context: [crates/nvs-codegen/src/lib.rs, crates/nvs-stdlib/src/instance.rs, crates/nvs-runtime/src/graph.rs, docs/rules/security/no-cross-request-state.md]
  - id: 3
    title: "every roster array is built at most once per request"
    status: open
    context: [crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-runtime/src/ctx/held.rs, crates/nvs-stdlib/tests/allocation_policy.rs]
  - id: 4
    title: "the rules, the reference, the examples, the feature proofs and the website say so"
    status: open
    context: [docs/examples/core/Reflect-PropertyInfo/name/about.md, website/src/data/changes.json]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: modifiers
    stage: 1
    name: "PropertyInfo::isLateInit and isReadonly answer each declaration, an inherited property included"
    run: [cargo, test, --lib, a_property_info_tells_]
    tests: [a_property_info_tells_lateinit, a_property_info_tells_readonly, a_property_info_tells_an_inherited_lateinit_property]
    timeout: 1800
  - id: meta
    stage: 1
    name: "nvs meta --json writes lateinit and readonly in a property signature"
    run: [cargo, test, --test, meta, a_property_signature_carries_]
    tests: [a_property_signature_carries_lateinit_and_readonly]
    timeout: 1800
  - id: once
    stage: 2
    name: "forClass, forObject and EnumInfo::of allocate nothing, and two calls return the same object"
    run: [cargo, test, --test, allocation_policy, a_description_]
    tests: [a_description_allocates_nothing, a_description_is_the_same_object_twice, a_description_of_an_enum_allocates_nothing]
    timeout: 1800
  - id: copy
    stage: 2
    name: "a description sent to a task or kept past a reload is never read through a freed unit"
    run: [cargo, test, --lib, a_copied_description_]
    tests: [a_copied_description_reads_its_own_unit, a_copied_description_of_a_reloaded_unit_throws]
    timeout: 1800
  - id: rosters
    stage: 3
    name: "a roster array is built once per request, and a write to it separates"
    run: [cargo, test, --test, allocation_policy, a_roster_]
    tests: [a_roster_is_built_once_per_request, a_roster_written_by_the_program_separates]
    timeout: 1800
  - id: proofs
    stage: 4
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, reflect-once]
    timeout: 1800
  - id: close
    stage: 5
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, reflect-once]
    timeout: 900
---

# Core\Reflect describes each class once per process, and PropertyInfo tells lateinit and readonly

## Why

Two asks, settled with the user on 2026-11-24.

**Reflection is slow because it rebuilds everything on every call.** `describe`
(`crates/nvs-stdlib/src/reflect.rs:1940`) builds a fresh `ClassInfo` plus one row object per
property, method, parameter, constant and attribute, and the arrays that hold them, on every
`forClass` and `forObject`. `describe_enum` (`reflect.rs:2068`) does the same for `EnumInfo::of`.
Measured on the release binary, 2026-11-24: one `forClass` of a class with 10 properties and 6 methods
costs about 1.65 µs, against about 1.4 ns for an empty loop step. A data mapper that describes its
class once per row pays that per row. Attributes are read only through this door
(`ClassInfo::attributes()`, `reflect.rs:2010`), so they gain the same.

**A program cannot tell a `lateinit` property by reflection.** `PropertyInfo` answers `name`,
`isPublic` and `type` (`reflect.rs:1508`), and `nvs meta --json` writes `?Customer $buyer` for both
`lateinit ?Customer $buyer;` and `?Customer $buyer = null;` (`crates/nvs-cli/src/meta.rs:1282`). A
framework's data mapper wants to refuse a reference property that is not `lateinit`, because only a
`lateinit` property loads on its first read (`rule:classes/late-init-initializes-on-the-first-read`).
The user widened it to `readonly` while the row changes anyway.

## Standing decisions

**A description is unit data, not request state.** It is a pure function of the compiled unit, as a
string literal is, so sharing it across requests and cores shares nothing a request wrote. The precedent
is the hole-free `` html`…` `` fold and the written duration: immortal objects every core reads
(`crates/nvs-runtime/src/object.rs:259`, emitter `crates/nvs-codegen/src/emit.rs:2133`).
`rule:security/no-cross-request-state` is not reversed; the rule fragment of stage 4 says why it does
not apply. The one thing a request can learn from it is nothing: the description is the same in every
request, and nothing is built during one, so there is no first-call timing either.

**Built when the unit is finished, never on first use.** `finish`
(`crates/nvs-codegen/src/lib.rs:3275`) calls a new `nvs_stdlib::reflect::build_descriptions(&mut
table)` before the table goes into its `Arc` (`lib.rs:3304`; the twin builder at `lib.rs:1710` too).
It walks every `ClassDesc` and `EnumDesc` and writes each description and every row with
`immortal_object_bytes` (`object.rs:4404`) and immortal string headers (`immortal_header_bytes`,
`crates/nvs-runtime/src/string.rs:422`). The bytes are owned by the `ClassTable` and freed with it.
`nvs-codegen` already depends on `nvs-stdlib`, so no crate edge is added. The storage type is
`nvs-runtime`'s, filled by `nvs-stdlib`. **What it spends:** a few KB per declared class, once per
process, for every class including one nothing reflects on, plus the build at unit load. The session
measures both, the bytes by a count, and writes the figures into the module doc.

**No word two cores can reach is ever written.** That is `object.rs:269`'s sentence, and it forces:
- **No array inside a shared object.** An array read writes its `RefCell` borrow flag
  (`crates/nvs-runtime/src/array.rs:121`). So `ClassInfo`'s rosters, `MethodInfo::parameters`,
  `AttributeInfo`'s names and values and `EnumInfo`'s cases and values leave the slots. The rows stay
  in a Rust-side table beside the description, and each roster member builds its array from them
  (stage 3 makes that once per request).
- **Every slot is a scalar or an immortal string.** An attribute value is a scalar constant
  (`constant_value`, `reflect.rs:2048`), so `AttributeInfo::field` reads the Rust-side table and needs
  no array.
- **No slot holds an address.** A description reaches its rows through a slot holding the class's id
  and a stamp of the unit that built it. A member called with a description whose stamp is not the
  calling request's unit throws a `LogicError`, because the id would name another class. This replaces
  the module doc's second decision (`reflect.rs:23`): its reason was a pointer that a reload leaves
  dangling, and a stamp fails closed instead.

**A copy stays correct.** The graph copy (`crates/nvs-runtime/src/graph.rs:727`) deep-copies an object
whose count is not 1 (`graph.rs:609`), so a description sent to a task arrives as an ordinary mortal
copy with the same id and stamp. A task that runs in the sending request's unit reads the same rows;
the session confirms which unit a task runs in before writing the test. A copy that outlives a reload
throws under the stamp rule. Stage 2 tests both, and if the graph copy
cannot copy an immortal object, it shares it instead, as an immortal string is shared.

**A `Core` class's description.** A `Core` descriptor is process-wide and not in the unit's table, and
`forObject` reaches one. It keeps the per-call build, and stage 3's memo makes it once per request.
`Core` classes declare no properties a walk names, so the build is a name and empty rosters.

**The roster memo is per request (stage 3).** `Ctx` gets one list of `(descriptor, roster kind) →
array`, built on the first `properties()`, `methods()`, `constants()`, `attributes()`, `parameters()`
or `cases()` call and handed out with a retain afterwards. It is freed with the request, like the
handle tables at `crates/nvs-runtime/src/ctx/mod.rs:1662`. A program that writes to the array it got
separates by copy-on-write (`array.rs:2884`), so the memo never changes. **What it spends:** one
array per roster per class described, per request, freed at its end, O(classes), never O(traffic).
`readableProperties` is not memoized: its answer depends on the calling site (`reflect.rs:46`).

**Identity.** `forClass('A') === forClass('A')` becomes true, and so does it for `forObject` on two
instances of one class. The rule fragment says so.

**The new members.** `PropertyInfo::isLateInit(): bool` is true for a property declared `lateinit`,
read off the descriptor's never-written marker (`FieldDefault::Unset`, `object.rs:1082`, in the
`defaults` list at `object.rs:466`), inherited properties included. `PropertyInfo::isReadonly(): bool`
reads `ClassDesc::field_is_readonly` (`object.rs:2100`). Both are bool slots on the row, built once
under stage 2. `Core\Reflect::steps` (`reflect.rs:2349`) rows carry them too. `nvs meta --json`'s
property signature writes `readonly` and `lateinit` before the type, in source order, as
`lateinit ?Customer $buyer`; `nvs code` shares that function (`crates/nvs-cli/src/code.rs:1805`).

**Declined.**
- **A per-request cache of whole descriptions.** It builds once per request per class. The per-process
  build costs nothing on the request path; the user chose it.
- **Building a description on its first use, behind a lock.** It is a first use on the request path,
  and it makes the first request after a reload slower than the rest.
- **Lazy rosters with a descriptor pointer in a slot.** The pointer is the hazard the stamp removes.
- **Other modifiers (`static`, a default value, an observer).** Nobody asked; each is a slot on every
  row.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is recorded in the handoff with what failed, and the stage goes on. If stage 2 finds the
per-process build unsound in a way this file does not cover, it stops and records why, and stage 3
becomes a per-request cache of whole descriptions instead; the user is told.

**Lanes.** Stage 1 lands first, because stage 2 builds rows that already carry its two slots. Stage 3
needs stage 2. In stage 4 the examples, the attack and the bench go to one writing subagent once the
rule is rewritten.

## Stages

### 1. PropertyInfo answers isLateInit and isReadonly, and nvs meta writes both modifiers

**Split:** none, one session.

- [ ] `crates/nvs-runtime/src/object.rs:2100`: `ClassDesc::field_is_lateinit(slot)` beside
  `field_is_readonly`, read off the `Unset` entries in `defaults`.
- [ ] `crates/nvs-stdlib/src/reflect.rs:1508`: two slots and two members on `PROPERTY_INFO`, their
  slot constants beside `reflect.rs:273`, natives beside `reflect.rs:3061`, the card and the member
  docs beside `reflect.rs:1552`; `describe` and `steps` fill both.
- [ ] `reflect.rs`'s tests: the three tests the check `modifiers` names, then
  `cargo test --lib a_property_info_tells_` once.
- [ ] `crates/nvs-cli/src/meta.rs:1282`: the modifiers before the type; the test the check `meta`
  names in `crates/nvs-cli/tests/meta.rs`, run once by name.

### 2. every description is built once when the unit loads, as immortal objects

**Split:** none, one session. Start by rewriting `reflect.rs`'s module doc decisions (`reflect.rs:5`
to `:50`) to this file's design, then build to them.

- [ ] `crates/nvs-runtime/src/object.rs:2519`: the `ClassTable` storage for descriptions and their
  rows, freed with the table, and the unit stamp.
- [ ] `crates/nvs-stdlib/src/reflect.rs:1940` and `:2068`: `build_descriptions` writes every
  description and row immortal; `describe` and `describe_enum` remain for `Core` descriptors only.
- [ ] `crates/nvs-codegen/src/lib.rs:3275` and `:1710`: both builders call it before the `Arc`.
- [ ] `reflect.rs:2306`, `:2424`, `:2968`: `forClass`, `forObject` and `EnumInfo::of` return the
  built description; every roster member and `AttributeInfo::field` read the Rust-side rows and check
  the stamp.
- [ ] `crates/nvs-stdlib/tests/allocation_policy.rs:142`: the three tests the check `once` names; the
  two tests the check `copy` names beside `reflect.rs`'s tests. Each run once by name.
- [ ] The bytes per class and the load time on `benches/members/core/Reflect/forClass.nvs`'s class,
  written into the module doc's *What it spends*.

### 3. every roster array is built at most once per request

**Split:** none, one session.

- [ ] `crates/nvs-runtime/src/ctx/mod.rs:1662`: the memo list on `Ctx`, freed with the request.
- [ ] `reflect.rs:2495`, `:2598`, `:2612`, `:2711`, `:2930` and `EnumInfo`'s rosters: build on the
  first call, retain afterwards. A `Core` descriptor's whole description goes through the same memo.
- [ ] `crates/nvs-stdlib/tests/allocation_policy.rs`: the two tests the check `rosters` names, run
  once by name.

### 4. the rules, the reference, the examples, the feature proofs and the website say so

**Split:** the examples, the attack and the bench go to one writing subagent once the rule is
rewritten; the session writes the rules and the website entries.

- [ ] `docs/rules/core-classes/reflect.md:23`: § *Memory* rewritten to the once-per-process build and
  the per-request roster memo, the identity sentence, why `rule:security/no-cross-request-state` does
  not apply, and the Declined items above.
- [ ] `docs/rules/core-classes/reflect.md:1`: `PropertyInfo` answers `isLateInit` and `isReadonly`.
  `docs/rules/tooling/meta-json-takes-a-program.md:1`: the signature carries both modifiers.
- [ ] `docs/examples/core/Reflect-PropertyInfo/isLateInit/` and `isReadonly/`: `about.md` and three
  examples each, written to AGENTS.md § *Text an end user reads*.
- [ ] The bench and the attack `bun nv proofs --owed --goal reflect-once` names, at the paths it
  prints; the attack tries to change a shared description or its rosters and reach another request.
- [ ] `website/src/data/changes.json:1`: one `added` entry for the two members and one `changed` entry
  for the speed of `Core\Reflect`. Each page reread and stamped with `bun nv site --stamp`.
- [ ] `bun nv proofs --owed --goal reflect-once` names nothing.

### 5. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of docs/agent/prose.md § *A newcomer reads it* over the pages stage 4 touched.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by the interactive session that settled the design with the user on 2026-11-24. Nothing is
built. Every anchor above was read that day.

### Next

Stage 1 from its first item. Every check is red until its stage's tests exist.

### Backlog
