---
looper: 1
title: a start copies no built-in manifest and parses no Core name twice
summary: "Reached when a start shares the built-in manifests without a copy, a program that names no built-in component parses none of them or the decline says why, the Core pre-pass parses each name once, the start-cost bounds are the measured figures, and the share of the start that the binary's size costs is recorded."
context: [crates/nvs-cli/src/extensions.rs, crates/nvs-ext/src/load.rs, crates/nvs-types/src/ext_lib.rs, crates/nvs-types/src/core_lib.rs, crates/nvs-cli/tests/start_cost.rs, docs/rules/programs/memory-priority.md, docs/rules/testing/perf-two-mechanisms.md]
stages:
  - id: 1
    title: the built-in manifests are shared, and the bounds are the measured figures
    status: done
    context: [crates/nvs-cli/src/extensions.rs, crates/nvs-ext/src/load.rs, crates/nvs-types/src/ext_lib.rs, crates/nvs-lsp/src/tree_cache.rs, crates/nvs-cli/tests/start_cost.rs]
  - id: 2
    title: a built-in component is parsed when a program names its namespace
    status: done
    context: [crates/nvs-cli/src/extensions.rs, crates/nvs-ext/src/builtin.rs, crates/nvs-ext/src/load.rs, crates/nvs-types/src/ext_lib.rs, crates/nvs-hir/src/symbol.rs]
  - id: 3
    title: "the `Core` pre-pass parses each name once"
    status: done
    context: [crates/nvs-types/src/core_lib.rs, crates/nvs-hir/src/qname.rs]
  - id: 4
    title: "the share of a start that the binary's size costs is recorded"
    status: done
    context: [tools/nv/cmd/start-cost.ts, crates/nvs-cli/src/main.rs]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: start-counted
    stage: 1
    name: "a small program's start does the counted work its bound allows, and no more"
    run: [cargo, test, --test, start_cost]
    timeout: 1800
    tests:
      - a_small_program_starts_within_its_counted_work
  - id: core-names-once
    stage: 3
    name: "every Core class still builds over the frozen types"
    run: [cargo, test, --lib, --, every_core_class_builds_over_the_frozen_types]
    timeout: 1800
    tests:
      - every_core_class_builds_over_the_frozen_types
---

# a start copies no built-in manifest and parses no Core name twice

## Why

`fast-start` cut one start of `nvs check` on a one-line program from about 34,700 allocations to
26,069 (the table is `crates/nvs-cli/src/main.rs`'s module doc, `# What it costs`). Two phases keep
work that the program does not use:

- `built_in`, 6,699 allocations (25.7%). Every start parses the JSON manifest and the source list of
  all four built-in components (`crates/nvs-ext/src/load.rs:763`). `with_built_in` then copies every
  parsed `Manifest`, each string in it included, into a new list
  (`crates/nvs-cli/src/extensions.rs:151`). `hir_classes` copies the method and constant names a
  third time (`crates/nvs-types/src/ext_lib.rs:80`).
- `core`, 8,109 allocations (31.1%). The type pre-pass (`crates/nvs-types/src/core_lib.rs:136`)
  must intern every `Core` type shape so type ids stay stable, and that stays. But `lower`
  (`core_lib.rs:794`) calls `QName::parse` (`crates/nvs-hir/src/qname.rs:40`: one `Vec` and one
  `String` per segment) on every `Instance`, `InstanceAt`, `Enum` and `EnumCase` row, and builds an
  argument vector per generic row, even when the interner already has the type. 4,948 of the 6,688
  pre-pass allocations are freed again at once. `build_class` (`core_lib.rs:176`) pays the same
  parses again for each class on its first lookup.

The clock will not move: both together are under a millisecond, and `--version` alone costs about
11 ms. That floor is process start and loading a 96 MB binary, which no goal has measured. Stage 4
measures it, so the next start goal is chosen by a figure.

`crates/nvs-cli/tests/start_cost.rs:14` still bounds each phase at stage 1 of `fast-start`'s
figures (`core` at 25,000, measured 8,109), so a regression up to three times the cost passes.

## Standing decisions

**Limits: what stops this goal.** Each line is a rule a session follows without asking.

- **Counts decide, the clock never does.** A change is judged by `bun nv start-cost`'s phase table.
  Two runs of one shape differ by about 4 ms, so no item is judged by its milliseconds.
- **The floor is `fast-start`'s:** a change to a start is built only where the phase gives it at
  least 5% of the start's allocations, one file read of 16 KiB or more, or one thread. Measure from
  `.agent-tmp/`, a directory with no `nvs.toml` of its own.
- **An item gets one attempt.** If its build or its named test is not green when its session ends,
  its uncommitted edits are discarded, the item is ticked as declined, and one line in `### Backlog`
  says what blocked it.
- **A stage gets two sessions.** `### State` holds one line per stage with the sessions used. At the
  ceiling, every open item of the stage moves to `### Backlog` as declined and the next stage starts.
- **No item is added.** A new finding goes to `### Backlog`. It never joins a stage.
- **Each stage lowers its phase's bound in the same commit** as its change: the measured figure plus
  10%, rounded up to the next hundred (`rule:testing/perf-two-mechanisms`).

**The built-in manifests are shared as `Arc<Manifest>`.** `with_built_in` returns
`Vec<Arc<Manifest>>`, and every reader of the list takes `&[Arc<Manifest>]` or derefs. An
`[[extension]]` entry's manifest is wrapped once where it is read. **What it spends:** one atomic
count per manifest, and a clone of the list is a count bump per entry. Safe default if a caller
needs an owned `Manifest` to change it: that caller clones its one entry, and the item says which.
Declined: borrowing `&'static Manifest` beside the program's own list, which splits every reader
into two iterators for the same saving.

**Stage 2 is built only behind one accessor, or declined.** A component is parsed the first time
name resolution asks for a name under its class's namespace, and `Stored::class`
(`crates/nvs-ext/src/builtin.rs:36`) gives the namespace without a parse. Every type, enum, resource
and source file of a component lives under that namespace (`load.rs:775` checks it), so the
namespace is a complete key. Built only if stage 1's measured `built_in` is still over the floor
**and** the load stays behind one call on the extension list, which the resolver and the checker
call in place of iterating it. Declined, with the number in `extensions.rs`'s `# What it costs`, if
it needs the collection phase or the checker to change how they walk `ExtensionClass` beyond that
call: **simplicity** outranks **memory footprint**, and the latency it buys is under a millisecond.
**What it changes:** a malformed built-in manifest panics at its first use, not at the start, so
item 2's test parses all four. In `nvs serve` the first compile runs before the first request, and
a recompile runs on the revalidation thread, so no request makes the parse. An editor completion
that lists every class parses all four, as today.

**The `Core` name table lives in the `Base`'s interner and is read-only after the pre-pass.** It
maps a row's `&'static str` name to its interned `TypeId` (`CoreName` in `crates/nvs-types/src/ty.rs`:
for `EnumCase`, the name and the case; for `InstanceAt`, the name and its argument slice's address
and length), filled on a miss in the pre-pass, and read through the layered interner by
`build_class` on each class's first lookup. A hit returns the id the interner already holds, and a
miss runs the code it runs now, so the order of interning and every type id stay the same. It sits
in the interner, beside its `aliases`, and not in a field of `Base` beside it, so a cached id always
names a type of the interner that gave it out: a fresh interner, as `seed` uses, starts with an
empty table, and `lower` keeps its signature. **What it spends:** one map of one entry per distinct
named `Core` type, held for the process in the `Base`. Declined: changing `QName` to shared
segments, which touches every crate for the same saving.

**Stage 4 measures and builds nothing.** It records what a start costs before `main` runs and what
the binary's size adds, on Linux in WSL (`bun nv linux-test`'s `novis-test` distro), where
`/usr/bin/time -f "%R %F"` counts minor and major page faults. Counts are compared with a small
release binary of the same workspace and toolchain, chosen by the `Explore` search the stage names.
Safe default if the distro has no `/usr/bin/time`: read `minflt` and `majflt` from
`/proc/<pid>/stat` in `start-cost`'s Linux path. A fix it suggests goes to `### Backlog`.

**Lanes:** one lane, in order. Stages 1 and 2 share `extensions.rs`, and every stage edits
`start_cost.rs`'s bounds.

## Stages

### 1. the built-in manifests are shared, and the bounds are the measured figures

**Does:** stop the copy of every built-in manifest at each start, and make the phase bounds catch a
regression.

**Split:** an `Explore` subagent lists every caller of `with_built_in`, `hir_classes`,
`seed_enums`, `ext_lib::signatures` and `live_manifests`, and every `Vec<Manifest>` or
`&[Manifest]` they pass on, as `file.rs:NN`, before the first edit.

- [x] `crates/nvs-cli/tests/start_cost.rs:14`: every bound is `fast-start`'s measured figure plus
  10%, before any other edit, and the doc comment names the figures
  (`rule:testing/perf-two-mechanisms`).
- [x] `crates/nvs-cli/src/extensions.rs:151`: `with_built_in` returns `Vec<Arc<Manifest>>` and copies
  no `Manifest` (`rule:programs/memory-priority`).
- [x] `crates/nvs-types/src/ext_lib.rs:80`: `hir_classes`, `seed_enums` and `signatures` take
  `&[Arc<Manifest>]`, and `crates/nvs-lsp/src/tree_cache.rs:63` holds the same list
  (`rule:programs/memory-priority`).
- [x] `crates/nvs-cli/tests/start_cost.rs:14`: `built_in`'s bound is the new measured figure plus
  10%, and `### State` records the figure for stage 2's gate (`rule:testing/perf-two-mechanisms`).

### 2. a built-in component is parsed when a program names its namespace

**Does:** a program that names no built-in component parses none of their manifests.

**Gate:** stage 1's recorded `built_in` figure is at least 5% of the start's allocations. Otherwise
every item below is ticked as declined with the figure.

- [x] `crates/nvs-cli/src/extensions.rs:143`: one `OnceLock` per `Stored` entry replaces
  `BUILT_IN_MANIFESTS`, filled by `nvs_ext::load` for that entry alone
  (`rule:packaging/the-first-party-components-are-built-in`). Declined: see `### State`.
- [x] `crates/nvs-hir/src/symbol.rs:163`: name resolution reaches a built-in class through one
  lookup by namespace on the extension list, which parses the component on its first call. If this
  needs more than that one call, the stage is declined as **Standing decisions** says
  (`rule:programs/memory-priority`). Declined: it needs four walks changed, not one call.
- [x] `crates/nvs-ext/tests/builtin_stored.rs:76`: `every_built_in_manifest_parses_on_its_own`
  parses each `Stored` entry through the new path
  (`rule:packaging/the-first-party-components-are-built-in`). Declined: no new path exists.
- [x] `crates/nvs-cli/tests/start_cost.rs:14`: `built_in`'s bound is the new measured figure plus
  10% (`rule:testing/perf-two-mechanisms`). Declined: the figure is stage 1's, 5,726.

### 3. the `Core` pre-pass parses each name once

**Does:** a `Core` type that is already interned costs a map lookup, not a `QName` and an argument
vector.

- [x] `crates/nvs-types/src/core_lib.rs:61`: the `Base` holds the name table **Standing decisions**
  describes (`rule:programs/memory-priority`).
- [x] `crates/nvs-types/src/core_lib.rs:794`: `lower` reads the table before it parses an
  `Instance`, `InstanceAt`, `Enum` or `EnumCase` name, and `build_class` at `:176` reads it too
  (`rule:programs/memory-priority`).
- [x] `crates/nvs-types/src/core_lib.rs:1254`: `every_core_class_builds_over_the_frozen_types` runs
  once, by name, and is green: the ids did not move.
- [x] `crates/nvs-cli/tests/start_cost.rs:14`: `core`'s bound is the new measured figure plus 10%
  (`rule:testing/perf-two-mechanisms`).

### 4. the share of a start that the binary's size costs is recorded

**Does:** measure what a start of `nvs` costs before its own code runs, so the next start goal is
chosen by a figure.

**Split:** an `Explore` subagent lists the release binaries the workspace builds, with their sizes,
before the first edit. The smallest one that links `std` is the comparison.

- [x] `tools/nv/cmd/start-cost.ts:1`: `--floor` runs `--version` of `target/release/nvs` and of the
  comparison binary in WSL, 25 interleaved runs each, and prints the median clock and the minor and
  major page faults of each (`rule:testing/perf-two-mechanisms`). The comparison is
  `nvs-serve-probe`, which has no `--version`, so it runs `--help`.
- [x] `tools/nv/test/start-cost.test.ts:1`: the Bun test of its argument handling covers `--floor`.
- [x] `crates/nvs-cli/src/main.rs:134`: `# What it costs` gets the floor figures and one sentence on
  what they leave for the binary's size.
- [x] `docs/goals/lighter-start.md:1`: `### Backlog` gets one line for each change the figures
  suggest, with its figure, or one line saying the size costs under 5% of `--version`'s faults.

### 5. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.
- [x] `crates/nvs-cli/src/main.rs:134`: run `bun nv start-cost` once and write its table into the
  module doc's `# What it costs`, with the `built_in` and `core` lines of what stays rewritten.

## Handoff

### State

Every stage is done, stage 2 as declined. Sessions used: stage 1: 1, stage 2: 1, stage 3: 1,
stage 4: 1, stage 5: 1. The goal is reached.

`looper verify` was green on its first run (716 s), so no fix and no second run were needed.
`bun nv start-cost`'s phase table is in `crates/nvs-cli/src/main.rs`'s `## What it costs`:
22,942 allocations in all, from 26,069. `built_in` 5,726, `core` 6,099, `check` 9,508. The
`built_in` and `core` bullets say what stages 1 to 3 changed.

The goal-end gates are green: `bun nv verify --doc`, `bun nv owners --closes lighter-start` (no
gap) and `bun nv ci --check`. Each standing decision's declined alternative now sits beside its
code: `with_built_in`'s doc in `extensions.rs` and `core_names` in `crates/nvs-types/src/ty.rs`.
The two floor suggestions of the backlog are under "Not built yet" in `main.rs`'s
`## What it costs`, so nothing is lost when this file is deleted.

The sweep's `env-secret-proofs` is stale state in looper's service, four sessions running:
`looper check --name env-secret-proofs` finds no check, no log exists, and the only mention in the
tree is this handoff. Nothing in the tree is red for it.

The refused DONE's `floor-debt` was one red proof,
`docs/examples/core/Storage/delete/03-remove-old-exports.nvs`. It listed the whole `files` disk,
which `01-delete-a-file.nvs` writes to when the two run at the same time. It now checks only its
own keys, and its output is unchanged in a run alone. `looper check --name floor-debt` is green.

The second refused DONE's `ci-check` was that fix's side effect: the edited example made
`/reference/core/storage/delete/`'s questions stale. The four questions still fit the page, so
the page is stamped again, and `bun nv ci --check` is green.

`cheaper-pipeline`'s goal-end push has not run: `main` is not pushed. Its runner-minutes are not
recorded anywhere yet.

### Next

- `docs/goals/lighter-start.md:1`: nothing is left. The loop retires the goal, and `after_retire`
  rebuilds `target/release/nvs`.

No test is expected to go red. A non-Windows runner's `start_cost` may count a few more
allocations than the 10% margin allows. If one does, measure on that OS and set the bound from
that figure.

### Backlog

- **The 3.2 MB `.data.rel.ro` costs about 787 of `nvs --version`'s 1,234 minor faults (64%) and
  0.4 ms per start.** Make the largest pointer tables pointer-free, for example the `Core`
  registry's rows as offsets into one string. Recorded in `main.rs`'s `## What it costs`.
- **`-z pack-relative-relocs`** shrinks the 3.8 MB `.rela.dyn` to a few KiB, and needs glibc 2.36
  on every Linux target a release supports. Recorded in the same place.
- Declined: a non-PIE build. It gives up ASLR, and **security** outranks **latency**.
