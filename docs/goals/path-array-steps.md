---
looper: 1
title: "a property path steps into an array's key and element with @key and @value"
summary: "Reached when a property path accepts @key and @value after any array property, in a written path, in a string<propertyPath<T>> hole and at run time, a property named key or value is still reached by its own name, Core\\Reflect::steps gives a row for each step, the editor completes and explains the two steps, and the rule, the reference and the feature proofs say so."
context: [crates/nvs-types/src/expr/members.rs, docs/rules/types/property-path.md]
stages:
  - id: 1
    title: the rule is written, and the checker steps into an array with @key and @value
    status: done
    context: [crates/nvs-types/src/expr_table.rs, docs/rules/types/string-holes.md, tests/conformance/lang/a-property-path-steps-through-objects-nullable-objects-and-arrays.nvst, tests/conformance/lang/a-misspelled-property-path-does-not-compile.nvst]
  - id: 2
    title: the run-time walk and Core\Reflect::steps take the two steps
    status: done
    context: [crates/nvs-runtime/src/helpers.rs, crates/nvs-ir/src/lower/convert.rs, crates/nvs-stdlib/src/reflect.rs, docs/rules/core-classes/str-parts-and-reflect-steps.md]
  - id: 3
    title: the editor completes, explains and jumps through the two steps
    status: done
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/definition.rs, crates/nvs-lsp/src/hover.rs, tests/lsp/completion/a-property-path-completes-one-segment-at-a-time.lspt]
  - id: 4
    title: the reference, the example and the attack say what the steps do
    status: open
    context: [docs/examples/lang/types/property-paths/about.md, docs/reference/lang/20-types.md, docs/agent/prose.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: cases
    stage: 2
    name: "@key and @value step into an array in a written path, in a string<K> hole and at run time, a property named key or value keeps its own name, Core\\Reflect::steps gives a row per step, and every wrong use does not compile or throws"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-property-path-steps-into-an-array-with-key-and-value.nvst, --case, tests/conformance/lang/a-property-path-with-key-and-value-converted-at-run-time.nvst, --case, tests/conformance/reject/a-misplaced-array-step-in-a-property-path-does-not-compile.nvst, --case, tests/conformance/core/reflect-steps-gives-a-row-for-an-array-step.nvst]
    timeout: 1800
  - id: editor
    stage: 3
    name: "after an array property in a property path, the editor offers @key, @value and the element's properties, shows what each step is, and jumps from @value.name to the property"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/completion/a-property-path-offers-key-and-value-after-an-array.lspt, --case, tests/lsp/hover/a-property-path-array-step-shows-its-type.lspt, --case, tests/lsp/definition/a-property-path-segment-after-value-jumps-to-its-property.lspt]
    timeout: 1800
    tests:
      - tests/lsp/completion/a-property-path-offers-key-and-value-after-an-array.lspt
      - tests/lsp/hover/a-property-path-array-step-shows-its-type.lspt
      - tests/lsp/definition/a-property-path-segment-after-value-jumps-to-its-property.lspt
---

# a property path steps into an array's key and element with @key and @value

## Why

A library that takes a condition as `string<propertyPath<T>>`, such as a query builder over stored
objects, writes each array property as a side table of keys and elements. A condition has to test an
element's key, and an element that is not an object. Today a path continues after an array only into a
property of its element class, so `{keywords.key}` and `{keywords.value}` on an `array<string>` are
`E0405`, and an `array<Author>`'s element itself cannot be named at all. A user asked for this, so the
library can write `{keywords.@value} IN (?)`, `{keywords.@key} = ?` and `{authors.@value} = ?`.

## Standing decisions

**The syntax (the person's call).** After a property whose type is an array, a path may take one of two
steps: `@key` names an element's key and ends the path; `@value` names the element itself and ends the
path or continues as from the element's type. The `@` keeps the two steps apart from properties: a
class's own `$key` or `$value` is still reached by its name, `{authors.value}`, and never changes
meaning when a property is added or removed. `@` means nothing else in Novis (`@expr` is only the
`E_SUPPRESSION_UNSUPPORTED` diagnostic, `crates/nvs-syntax/src/parser/expr.rs:880`), and it cannot be a
property name.

**Which types take the steps (my call).** `array<E>` and `?array<E>`, for any `E`. `@value` steps to
`E`: a class `C` or `?C` continues into `C`'s properties, an array takes `@key`/`@value` again
(`{grid.@value.@key}`), and anything else ends the path. The existing shorthand stays: a property of the
element class directly after an array property, `{authors.name}`, is `{authors.@value.name}`. A
`?array<C>` steps like `array<C>`, shorthand included. The shorthand applies after every array a path
reaches, so `grid.@value.name` is `grid.@value.@value.name`, and an `array<?C>` element counts as a
class: both fall out of one rule, `path_step` applied to `E`, with no special case. `@key` after a non-array, `@value` after a non-array, a
step after `@key`, and any other `@` word (`@Key`, `@index`, `@`) is the same unknown-member diagnostic
(`E0405`), placed on that segment, with its own message: "`$name` is not an array, so the path cannot
take `@key` after it" and "an array's steps are `@key` and `@value`".

**The step rule has one home (my call).** `path_step_class`
(`crates/nvs-types/src/expr/members.rs:882`) becomes a function that returns what a step reaches: a
class, an array with its element type, or nothing. The checker, the graph builder, the editor
(`crates/nvs-lsp/src/completion.rs:935`, `crates/nvs-lsp/src/definition.rs:1127`) and the roster
recorder (`members.rs:941`) all read it, so the editor and the checker step the same way.

**The run-time graph (my call).** `PathNode` (`crates/nvs-types/src/expr_table.rs:379`) keeps its shape.
A step into an array leads to an array node, one per distinct array type reached. Its class line is the
rendered type, such as `array<Author>`, which a throw names. Its rows are `@key` (no next), `@value`
(next is the element's node, or none), and, when the element is a class, a copy of that class's rows for
the shorthand. `encode_path_graph` (`crates/nvs-ir/src/lower/convert.rs:2356`) and the walk
(`crates/nvs-runtime/src/helpers.rs:3322`) are unchanged, because an array node is an ordinary node. The
throw messages (`helpers.rs:3420`, `:3526`) name an array node's segment as the checker does.

**`Core\Reflect::steps` (my call).** A `@key` step is a `PropertyInfo` row named `@key`, `isPublic`
`true`, type `int|string`. A `@value` step is a row named `@value`, `isPublic` `true`, with the element's
declared type rendered, such as `Author` or `string`. A shorthand segment is the element class's own
row, as today. No new class: `rule:core-classes/str-parts-and-reflect-steps` already declines a
`PathStep`. A library tells an array step from a property by the `@`. The element class for a shorthand
row on an array node is the class line of the node its `@value` row leads to.

**Security.** A path is still declared public names plus two fixed words, so `as propertyPath<T>` and the
holes of `as string<K>` still strip `tainted` soundly (`rule:types/property-key`). No step reads a value.

**What it costs.** Nothing at run time for a program that writes no `@` step: the walk reads one roster
per segment, as now. The encoded graph, a compile-time constant, gains one node per distinct array type
a path can reach, with the element class's rows copied into it. `Core\Reflect::steps` still allocates
one row per segment, per call.

**Where it is written down.** `docs/rules/types/property-path.md` gets the steps, the types that take
them and a Declined entry: **unprefixed `key` and `value`**, because whichever of the step and a property
of that name wins, the other cannot be reached, or a path changes meaning when a property is added.
`docs/rules/core-classes/str-parts-and-reflect-steps.md` gets the two rows. The feature is part of
`lang:types/property-paths`, so its proofs are added there.

**Lanes.** Stages 1 and 2 run in order: stage 2 reads the graph stage 1 builds. Stage 3 changes
`crates/nvs-lsp/` alone and stage 4 changes `docs/` and `tests/hostile/` alone. Both need stage 2 and
may run side by side after it.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. the rule is written, and the checker steps into an array with @key and @value

**Does:** writes the rule and builds the keystone: one step rule the checker and the graph builder read.

- [x] `docs/rules/types/property-path.md:6`: the step paragraph gains `@key` and `@value`, the types that
  take them, the shorthand and `?array<E>`, written from § *Standing decisions*, with the Declined entry
  for unprefixed `key` and `value`. `data/rules/types/property-path.json` follows. Then
  `bun nv rules --render`, and `bun nv rules --check` is green.
- [x] `crates/nvs-types/src/expr/members.rs:882` `path_step_class`: returns a step target, a class or an
  array with its element `TypeId`, for `C`, `?C`, `array<E>` and `?array<E>`. Every caller
  (`bun nv peek --locate path_step_class`) reads it, the LSP callers included, so the tree builds.
- [x] `crates/nvs-types/src/expr/members.rs:1000` `check_path_segments`: walks a step target. On an array
  it takes `@key` (last segment only), `@value`, or a property of a class element. Every wrong use is
  `E0405` at its segment with the messages in § *Standing decisions*, held to `docs/agent/prose.md`.
- [x] `crates/nvs-types/src/expr/members.rs:910` `property_path_graph` and `:941`
  `record_path_property_keys`: an array node per distinct array type, as § *Standing decisions* says.
- [x] `tests/conformance/lang/a-property-path-steps-into-an-array-with-key-and-value.nvst:1`: `@key`
  and `@value` on an `array<string>`, an `array<Author>`, an `array<array<Author>>` and a
  `?array<Author>`, the shorthand beside `@value.name`, and a class with its own `$value` reached by
  `value`; each as a `propertyPath<T>` literal and as a `string<propertyPath<T>>` hole, printed.
- [x] `tests/conformance/reject/a-misplaced-array-step-in-a-property-path-does-not-compile.nvst:1`:
  `@key` after a non-array, a step after `@key`, `@Key`, `@index` and a lone `@`, each one `E0405` at
  its segment.

### 2. the run-time walk and Core\Reflect::steps take the two steps

**Does:** a path converted at run time and `Core\Reflect::steps` read the new array nodes.

- [x] `crates/nvs-runtime/src/helpers.rs:3420` and `:3526`: a throw for a bad segment on an array node
  says what the checker says. The walk itself (`:3322`) needs no change; confirm with the case below.
- [x] `crates/nvs-stdlib/src/reflect.rs:2257` `nvs_core_reflect_steps`: a `@key` or `@value` segment
  gives the row in § *Standing decisions*; a shorthand segment on an array node is looked up on the
  element class its `@value` row leads to. `STEPS_DOC` (`:435`) and the reference card say so, held to
  `docs/agent/prose.md`.
- [x] `docs/rules/core-classes/str-parts-and-reflect-steps.md:11`: the two rows. `bun nv rules --render`.
- [x] `tests/conformance/lang/a-property-path-with-key-and-value-converted-at-run-time.nvst:1`: the
  stage 1 paths through `as propertyPath<T>` and `as string<K>`, and the wrong ones through
  `as ?propertyPath<T>` (`null`) and `as propertyPath<T>` (the throw, printed).
- [x] `tests/conformance/core/reflect-steps-gives-a-row-for-an-array-step.nvst:1`: name, `isPublic` and
  type of each row for `keywords.@key`, `keywords.@value`, `authors.@value.name` and `authors.name`.
- [x] `crates/nvs-stdlib/src/reflect.rs:6719`: a Rust test beside
  `steps_reads_each_segment_from_the_class_the_walk_reached`, for a path through an array node.
- [x] The `cases` check runs once.

### 3. the editor completes, explains and jumps through the two steps

**Does:** the editor steps with the stage 1 rule.

- [x] `crates/nvs-lsp/src/completion.rs:935` `path_steps`: after an array property it offers `@key`,
  `@value` and, for a class element, the element's properties. After `@value` it offers what the element
  type takes.
- [x] `crates/nvs-lsp/src/definition.rs:1127` `property_segment_at`: steps over `@value`, so go-to-definition,
  hover and rename reach a property after it. `@key` and `@value` jump to the array property they follow.
- [x] `crates/nvs-lsp/src/hover.rs:656` `property_segment`: hovering `@key` shows "the key of an element
  of `$keywords`" and `int|string`; `@value` shows the element type. End-user text.
- [x] `tests/lsp/completion/a-property-path-offers-key-and-value-after-an-array.lspt:1`,
  `tests/lsp/hover/a-property-path-array-step-shows-its-type.lspt:1` and
  `tests/lsp/definition/a-property-path-segment-after-value-jumps-to-its-property.lspt:1`, modelled on
  the `a-property-path-*` cases beside each. The `editor` check runs once.

### 4. the reference, the example and the attack say what the steps do

**Does:** the user pages. Every line here is text an end user reads, held to `AGENTS.md` § *Text an end
user reads*. Run `bun nv proofs --id lang:types/property-paths` first and last.

**Split:** one writing subagent writes the example and the attack while the session writes the
reference and `about.md`. Their files are disjoint, and no one builds until both end.

- [ ] `docs/examples/lang/types/property-paths/about.md:1`: one paragraph. After an array property, `@key`
  is an element's key and `@value` is the element. A property named `key` or `value` keeps its name.
- [ ] `docs/reference/lang/20-types.md:3245`: the same, with `Core\Reflect::steps`'s two rows.
- [ ] `docs/examples/lang/types/property-paths/06-a-condition-on-an-array-key-and-element.nvs:1`: a
  `find` method that takes `string<propertyPath<static>>`, called with `@key`, `@value` and the
  shorthand, printing `Core\Reflect::steps` for each hole.
- [ ] `tests/hostile/lang/types/property-paths/03-array-steps-from-a-request.nvs:1`: request text tries
  `@key` after a non-array, a step after `@key`, `@` words that are not steps and a private property
  after `@value`. Each gives `null` from `as ?propertyPath<T>`.
- [ ] `bun nv proofs --id lang:types/property-paths` reports the feature complete, and
  `bun nv proofs --comments` passes on the two new files.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1, 2 and 3 are done. The `cases` and `editor` checks are green.

The editor steps with the checker's rule. `property_segment_at` (`crates/nvs-lsp/src/definition.rs`)
walks `@key`, `@value` and properties through `nvs_types::path_step`, and a `PathSegment` carries an
`ArrayStep` for a step. A step answers the array property it follows: `definition::at` jumps there,
and `named_at` skips a step, so references and rename never treat `@key` as the property. Hover
(`hover.rs` `property_segment`) shows `int|string` and "The key of an element of `$keywords`.", or
the element type and "An element of `$authors`."; after another `@value` it names the array type.
Completion (`completion.rs` `path_steps`, `path_after`) offers `@key` and `@value` as keywords, then
the element class's properties; `open_hole` accepts `@` in a `string<propertyPath<T>>` hole.
`completion_files::Value::step` builds the two items.

The sweep's `env-secret-proofs` red is stale: its log does not exist, and `git log -S` names only
this goal's handoff and `in-place-everywhere`'s retire commit.

### Next

Stage 4 from its first item: `docs/examples/lang/types/property-paths/about.md:1`,
`docs/reference/lang/20-types.md:3245`, the example `06-a-condition-on-an-array-key-and-element.nvs`
and the attack `tests/hostile/lang/types/property-paths/03-array-steps-from-a-request.nvs`, per the
stage's split. Run `bun nv proofs --id lang:types/property-paths` first and last. The website's
`core.json` and `llms-full.txt` embed the `steps` card and `about.md`, so `bun nv render --website`
is owed before the gate. `Core\Reflect`'s perf records are stale after stage 2: record them last,
per the playbook. Expect red at the gate until stage 4 lands: the proofs gate for
`lang:types/property-paths`.

### Backlog
