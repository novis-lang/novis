---
looper: 1
title: "completion offers only the types a file's program can load"
summary: "Reached when the editor's type completion, its Ns\\ lists, its use lines, its new and its unloaded extend blocks offer only declarations the file's autoload map can load, plus Core, while an autoload string still offers every namespace, references stay workspace-wide, a file no program loads keeps the whole list, and the rule, the editor page and the changelog say so."
context: [docs/rules/ide/a-bare-name-reaches-every-type-and-imports-the-one-accepted.md, docs/rules/ide/an-autoloaded-file-borrows-its-programs-map.md, docs/rules/programs/autoload.md, crates/nvs-lsp/src/completion.rs, crates/nvs-hir/src/autoload.rs]
stages:
  - id: 1
    title: "the map says whether it loads an indexed declaration, and every type list the editor offers asks it"
    status: done
    context: [crates/nvs-lsp/src/document.rs, crates/nvs-lsp/tests/autoload.rs, docs/rules/ide/completion-offers-only-what-the-compiler-derived.md]
  - id: 2
    title: "the rules, the editor page and the changelog say so"
    status: done
    context: [website/src/content/docs/tools/editor/index.mdx, website/src/data/changes.json, docs/rules/ide/check-scope-defaults-to-the-workspace.md]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: loadable
    stage: 1
    name: "in a workspace of modules, each module's type lists offer only what its map loads, plus Core, an autoload string offers every namespace, references reach every module, and a file no program loads gets every type"
    run: [cargo, test, --test, loadable_completion]
    tests: [a_bare_name_offers_no_type_of_another_module, a_bare_name_offers_the_framework_and_vendor_types_the_map_loads, a_namespace_list_offers_only_loadable_types, a_use_line_offers_only_loadable_types, new_offers_only_loadable_classes, an_unloaded_extend_block_of_another_module_is_not_offered, an_autoload_prefix_still_offers_every_namespace, a_links_root_type_is_not_offered_at_a_bare_name, a_file_no_program_loads_is_offered_every_type, references_still_reach_every_module]
    timeout: 1800
  - id: proofs
    stage: 2
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, loadable-completion]
    timeout: 1800
  - id: close
    stage: 3
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, loadable-completion]
    timeout: 900
---

# completion offers only the types a file's program can load

## Why

A module's entry file declares what the module may load: the framework, the vendor packages and its
own root, through its `autoload` map (`rule:programs/autoload`). The compiler holds the module to
that map. A name no root reaches is undeclared, and with no runtime loader
(`rule:programs/no-runtime-autoload`) another module's class is not in the compiled program at all.

The editor ignores the map when it offers a type. A bare name, a `Ns\` list, a `use` line and `new`
are offered every declaration in the workspace index (`every_type`,
`crates/nvs-lsp/src/completion.rs:4530`; `under`, `:3172`). In a repository of many modules, each
module's lists hold every other module's types. Accepting one writes a `use` line, and the file then
does not compile. The editor offers code the compiler refuses, which
`rule:ide/completion-offers-only-what-the-compiler-derived` exists to prevent.

The user raised it on 2026-11-26 as a per-module allowlist or blocklist of namespaces. The map
already is that allowlist and the compiler already enforces it, so this goal adds no list: the editor
reads the map it already holds for the file (`Analysed::autoload`,
`crates/nvs-lsp/src/document.rs:844`, borrowed per
`rule:ide/an-autoloaded-file-borrows-its-programs-map`).

## Standing decisions

**What "loadable" means.** A declaration in the workspace index is loadable from the cursor's file
when the program already loaded it (`analysed.module.symbols`), or when the file's map resolves the
declaration's name to the declaration's own file. The second test is a path comparison, never a disk
probe: the name's longest matching prefix among the ordinary entries, one of that prefix's roots,
the remaining segments as directories and the last one plus `.nvs`, compared with the indexed path.
It is a new `AutoloadMap::loads_file(&self, name: &QName, path: &Path) -> bool` beside `resolve`
(`crates/nvs-hir/src/autoload.rs:596`), sharing its prefix walk, so the editor and the compiler read
one implementation of the map. Roots are compared in the form the index stores paths in; the map's
roots are made comparable once per analysis, never once per item. Fallback if two roots of one prefix
both hold the file: offer it, since the label is keyed by name and appears once.

**`Core` is always offered.** No program autoloads into it.

**An `autoload links` root loads nothing** (`rule:programs/autoload`), so its types are not offered at
a bare name, a `Ns\` list, `use` or `new`. The route argument of `Core\Router::url` keeps reading
`loadable_links` (`completion.rs:1516`) as today.

**What changes.** Every list that offers a workspace type by name:
- `every_type` (`completion.rs:4530`), which the bare-name, `use` and `new` lists read (`:4448`).
- `under` (`completion.rs:3172`), the `Ns\` list (`:464`).
- `unloaded_blocks` (`completion.rs:810`): an `extend` block of another module is offered as members
  only when the map loads its file.

**What stays workspace-wide.**
- The namespaces offered inside an `autoload` prefix or path string (`prefixes`, `completion.rs:1447`).
  That is where a developer adds a dependency, so every module stays findable there.
- `taken_short_names` (`completion.rs:4554`) keeps the whole list. A `use` line must not take a
  short name any type in the namespace in force answers to, loadable or not.
- Find references, rename, the type hierarchy, the code lens counts and the unused-member dimming:
  `rule:ide/five-features-are-one-reference-index`. Changing code across modules needs every module.

**A file with an empty map gets the whole list, as today.** That is a file no program loads, a new
class file before any entry maps it, and a program that only uses `require`. An empty list there
would offer nothing a developer can start from. "Empty" means no ordinary entry
(`AutoloadMap::loads_nothing`): a map of `autoload links` lines alone loads nothing, so it gets the
whole list too. A type declared inside a class (`Shop\Cart::State`) is tested by its class's name,
since it is loaded with that class.

**No setting.** The map is the one statement of what a module may load. A second list would say the
same thing twice and drift, and a blocklist would fail open for every module added later.

**Cost.** Each type list does one prefix walk and one path comparison per indexed type, where it
already builds one item per type. A module's lists get shorter, so each answer is smaller to send
and for the editor to filter. Memory: the comparable roots, O(roots) per analysis, built and dropped
with it. Nothing on a request path.

**Rules.** `rule:ide/a-bare-name-reaches-every-type-and-imports-the-one-accepted` is rewritten whole to
say "every type the file's program can load", with the loadable test, the fallback and the lists
that stay workspace-wide. Its id stays, since a rename is not needed for a narrowing; its title in
`docs/ground-rules.md` changes through `bun nv rules`. `rule:ide/check-scope-defaults-to-the-workspace`
says the index is what completion offers a type out of, and gains "that the file's program can load".

**Declined, written into the rewritten rule's Declined list.**
- **A per-module allowlist or blocklist of namespaces in a file of its own.** It states what the map
  already states, the two drift, and the editor would then hide code that compiles or offer code
  that does not. A blocklist also fails open for every new module.
- **Sorting another module's types last instead of leaving them out.** Each one is a compile error
  where it is offered.
- **Probing the disk per item through `AutoloadMap::resolve`.** One file-system call per indexed type
  per request, for an answer the index's path already gives.
- **Narrowing references and rename to the map.** A change to a framework class must reach every
  module that uses it.

**Security.** This is not a security boundary and the rule says so. The boundary is the compiler and
the map, and it already holds. A module that must not be used from other modules is held by a
`[[restrict]]` entry (`rule:packaging/a-restriction-limits-where-a-declaration-is-used`).

**Feature proofs.** This changes the editor's completion, which has no proof per list. `bun nv proofs
--owed --goal loadable-completion` names whatever the work reached; if it names nothing, the check
`proofs` is green with nothing written.

**Website.** One `website/src/data/changes.json` entry of kind `changed` for type completion, written
to AGENTS.md § *Text an end user reads*. `website/src/content/docs/tools/editor/index.mdx:20`
(§ *Completion and errors*) gets two or three sentences: completion offers the types your program can
load, and you add a module to the list with an `autoload` line. Neutral names only (`Shop`, `Blog`,
`Framework`).

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is written to `### Backlog` with what blocked it.

**Lanes.** It runs on the chain after `refactor-commands` and before `test-only`. All three edit
`crates/nvs-lsp/src/completion.rs`, so they run in order and never side by side.

## Stages

### 1. the map says whether it loads an indexed declaration, and every type list the editor offers asks it

- [x] `crates/nvs-hir/src/autoload.rs:596`: `AutoloadMap::loads_file` beside `resolve`, over the
  ordinary entries only, sharing the prefix walk; with a unit test beside the map's own tests for a
  hit, a file under the wrong root, a longer prefix that wins, a links root and a `discover` root.
  `rule:programs/autoload`.
- [x] `crates/nvs-lsp/src/completion.rs:4530`: `every_type` takes the cursor's analysis and keeps a
  workspace declaration only when it is loadable, and only when the map is not empty; `Core` always.
  Its callers (`:4448`) pass it, and `taken_short_names` (`:4554`) keeps the whole list.
- [x] `crates/nvs-lsp/src/completion.rs:3172` and `:810`: `under` and `unloaded_blocks` apply the same
  test. `prefixes` (`:1447`) does not change.
- [x] `crates/nvs-lsp/tests/loadable_completion.rs` (new, with its workspace set up as
  `crates/nvs-lsp/tests/autoload.rs` sets up a program and its map): a `Framework` folder, two modules
  `Shop` and `Blog` whose entries map `Framework` and their own `src`, a vendor package line, a links
  root, and one class file no program loads. The ten tests of the check `loadable`, then
  `cargo test --test loadable_completion` once.
- [x] Any existing completion test whose workspace now offers fewer types is updated in the same
  slice; the session names each in the commit body.

### 2. the rules, the editor page and the changelog say so

- [x] `docs/rules/ide/a-bare-name-reaches-every-type-and-imports-the-one-accepted.md` rewritten whole
  and `docs/rules/ide/check-scope-defaults-to-the-workspace.md:4` per § *Rules*, then `bun nv rules`.
- [x] `website/src/content/docs/tools/editor/index.mdx:20`: the sentences of § *Website*, then
  `bun nv site --stamp` on it.
- [x] `website/src/data/changes.json:1`: the `changed` entry, then `bun nv site --check stale`,
  `changes` and `review` green.
- [x] `bun nv proofs --owed --goal loadable-completion` names nothing.

### 3. the goal is verified once, and every failure is fixed

- [x] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [x] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Every stage is done. `cargo test --test loadable_completion` is ten of ten green, and the work is
`AutoloadMap::loads_file` (`crates/nvs-hir/src/autoload.rs`) and `loadable` in `completion.rs`,
which `every_type`, `under` and `unloaded_blocks` read. The rule, the editor page and one
`changes.json` entry say so. Every call under § *Standing decisions* is open until the user changes
one.

**`DONE` waits on the user's `E0454` work, which is still being edited.** Its code landed in
c8eca50b11 at 09:42 on 2026-11-27. Its `about.md`, the reference chapter, a reject case and its
`changes.json` entry are still uncommitted at 09:44. At 09:44 the tree builds and is formatted, and
`loadable` is ten of ten green. `close` is red only on that work's stamps: `llms-full.txt` not
rendered and 14 `lang:expressions/*` perf proofs stale on `30-expressions.md`. No session stamps
another person's unfinished work. The nightly run 45813627231's three causes are fixed on `main`
after it: c115a735a1 (zone tests skip Miri), 28a4d7a6bf (`Core\Uuid` is `Parses` at run time) and
544711a8fb (the waiting-stack bound covers aarch64's 11.03). No test is expected to go red.

### Next

Nothing in this goal. Once the `E0454` change is committed with its stamps and its `changes.json`
entry and formatted, `bun nv close --goal loadable-completion` is green and `DONE` is claimed again. If that work
is abandoned, discarding it is the user's decision, never a session's.

### Backlog
