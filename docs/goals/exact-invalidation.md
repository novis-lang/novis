---
looper: 1
title: an edit re-checks only the code that can see what changed
summary: "Reached when an edit to a body, a comment or a new member nobody names re-checks no other file, a private member re-checks only the subclasses and the files that decode into the class, a protected member only the subclasses, a public member only the files that use it or failed to find it, and a counted test pins each case on a generated tree of 200 entry points."
context: [crates/nvs-lsp/src/surface.rs, crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/session.rs, docs/rules/ide/the-index-is-incremental-and-persistent.md]
stages:
  - id: 1
    title: "one hash per declaration, and the diff of two parses"
    status: done
    context: [crates/nvs-lsp/src/parse_cache.rs, crates/nvs-lsp/src/disk_index.rs, crates/nvs-syntax/src/ast.rs]
  - id: 2
    title: "the checker records what each file used and failed to find"
    status: open
    context: [crates/nvs-types/src/partial.rs, crates/nvs-types/src/check.rs]
  - id: 3
    title: "a file's checked output is keyed on what it used"
    status: open
    context: [crates/nvs-lsp/src/document.rs, crates/nvs-types/src/partial.rs, crates/nvs-lsp/tests/incremental.rs]
  - id: 4
    title: "an edit queues only the entries that own a file it reaches"
    status: open
    context: [crates/nvs-lsp/src/disk_index.rs, crates/nvs-lsp/src/progress.rs, crates/nvs-lsp/tests/incremental.rs]
  - id: 5
    title: "the rule, the measurement and the change entry"
    status: open
    context: [tools/nv/cmd/lsp-start.ts, website/src/content/docs/tools/editor/index.mdx, website/src/data/changes.json]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: reach
    stage: 4
    name: "each kind of edit re-checks and re-queues exactly the files that can see it, counted on a tree of 200 entry points"
    run: [cargo, test, --test, reach]
    timeout: 1800
    tests:
      - a_private_member_edit_reaches_no_other_file
      - a_new_member_nobody_names_reaches_no_other_file
      - a_new_member_reaches_the_file_that_failed_to_find_it
      - a_new_member_reaches_the_subclass_that_declares_its_name
      - a_new_abstract_or_interface_method_reaches_every_implementer
      - a_protected_member_edit_reaches_only_the_subclasses
      - a_public_member_edit_reaches_only_its_users
      - a_private_property_edit_reaches_the_files_that_decode_into_the_class
      - a_class_header_edit_reaches_every_user_of_the_class
      - a_removed_member_reaches_its_users
      - a_graph_edit_reaches_every_reader
  - id: oracle
    stage: 3
    name: "a cached analysis reports what a fresh one does after every kind of edit, each visibility included"
    run: [cargo, test, --test, incremental]
    timeout: 1800
    tests:
      - a_cached_analysis_reports_what_a_fresh_one_does_after_every_kind_of_edit
  - id: close
    stage: 6
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, exact-invalidation]
    timeout: 900
---

# an edit re-checks only the code that can see what changed

## Why

The user reported on 2026-11-23 that in a big project one keystroke in the editor shows "Indexing … x of
200 files". A probe of `target/release/nvs.exe lsp` over a 972-file tree (`bun nv lsp-start --copies`)
measured it:

- A space inside a method body re-queued nothing. That case was already fixed.
- A space between two members, or in a signature, re-queued every entry point that read the file. Each
  of those analyses checked **every body of its whole graph** again: `checked` went 972 → 1946 → 2918,
  one whole tree per keystroke.

Two things in the code cause it:

1. `SymbolIndex::refresh` (`crates/nvs-lsp/src/index.rs:807`) compares **one** surface hash for the
   whole file. Any difference queues every entry from `readers_of` (`index.rs:1376`).
2. A file's checked output is keyed per **graph** (`graph_key`, `crates/nvs-lsp/src/document.rs:1442`;
   `Session::keys`, `crates/nvs-lsp/src/session.rs:148`). It hashes every file's surface, so one
   changed surface re-checks every file of every graph that read it.

Commit `1117cd5bbc` took whitespace and plain comments out of the surface hash. That is the first,
smallest step. This goal does the rest, following the user's rule: an edit reaches only the code that
can see it. A private member reaches no other file. A new member reaches no other file unless that file
failed to find the name, or declares the same name in a subclass. A protected member reaches only the
subclasses. A public member reaches only the files that use it.

`rule:ide/the-index-is-incremental-and-persistent` declines "a per-file dependency set in the check
output's key" because a surface edit was thought rare. The user overruled that on 2026-11-23, so stage
5 removes that Declined entry and states the new behaviour.

## Standing decisions

**The unit of reach is a symbol, written the way the index already writes it**: `App\User`,
`App\User::greet`, `App\User::$name`, `App\User::LIMIT` and an enum case `App\Status::Draft`
(`crates/nvs-lsp/src/index.rs` module doc, "a symbol is a string"). Novis has no top-level function or
constant (`rule:classes/no-free-functions-or-constants`), so there is no other kind. The symbols are interned per session, so a uses set is a set of ids. Reason: one name per
symbol across the index, the surface rows and the uses. A second scheme would need a map between them.

**A surface row is one declaration**: its doc comment, attributes, modifiers, visibility, type, default
and signature, written as `surface::of` writes them now (whitespace and plain comments out). A class's
own row is its header: doc, attributes, `abstract`/`final`/`readonly`, `extends`, `implements`, and the
**set of its enum cases** for an enum. The file's graph row is its `namespace`, `use`, `autoload` and
`require` lines and every file-scope statement whose text has `require`. `surface::of` stays the hash of
all the rows, so its callers keep working.

**What a diff reaches**, for each symbol that is added, removed or changed:

| What changed | Files it reaches, besides its own |
|---|---|
| a private method or constant | the subclasses: the finality and redeclaration checks read it |
| a private property | the subclasses, and every file with a JSON or database codec site that reaches the class |
| a protected member | the files that declare a subclass of the class, at any depth |
| a public member or enum case | the files that used the symbol, and the subclasses |
| an added member, case or type | the files that failed to find that name, and the subclasses |
| an added abstract or interface method | every implementer and subclass |
| a class header | every file that used the class or any of its members, and the subclasses |
| a removed symbol | the files that used it, and the subclasses |
| a member's attributes | every reader of the file, as today: routes, commands and links are whole-graph tables |
| the graph row | every reader of the file, as today |

A subclass is reached by any non-private change in a parent, because the override, abstract and
interface checks land in the subclass's file. **Safe default:** where stage 2 finds a check whose
result depends on something this table does not list, that thing counts as part of the class header.
Too wide costs time, too narrow gives a wrong answer.

**Two rows the table did not name, and what each reaches** (stage 1, `crates/nvs-lsp/src/surface.rs`
module doc). A **properties row** per class is the names of its properties, promoted ones included, in
order, at private visibility: the derived codecs read every property in declaration order, so an added,
removed or moved property of any visibility reaches the codec files and the subclasses through it, and
an added public property reaches its codec files although "an added member" does not list them. A
**cases row** per enum is its case names in order, at public visibility: stage 4 gives it the reach of
"an added member" (the failed lookups and the subclasses) unless stage 2 finds a check that reads the
set. A case without a value has the last written value and its distance from it in its row, so an
inserted case changes the rows of every case whose value moved. A member's attributes are a second hash
on its row, and the diff reports them as kind `Attributes`; a class's own attributes are the same, so
they reach every reader too. A method that gains or loses a body changes kind (`Method`,
`AbstractMethod`), which the diff gives as one removal and one addition: a superset of a change's reach.

**A constant read through another constant is a use of both** (for stage 2). A class constant or a
case value is inlined at each use, and `B = self::A + 1` is inlined with A's value in it, but B's row
holds only B's text. So the file that reads B must record A as a use too, or an edit to A leaves its
cached output stale. Stage 2 records the constants a constant's value reads, transitively, at each use.

**Private stays private except where another file's check reads it.** A read of the checker on
2026-11-23 found these, and the table's two private rows follow from them:

- The derived JSON and database codecs take every property, private ones and ancestors' included
  (`rule:core-classes/derive-property-list`; `field_order`, `crates/nvs-types/src/derive.rs:2957`;
  `chain_field_order`, `derive.rs:2987`). A `decodeAs<T>` in one file depends on T's private property
  types in another. The codec sites are already per file in `FileOutput` (`partial.rs:73`), and
  `classes_reached` (`derive.rs:2367`) gives the classes a site reaches.
- Constructor initialisation reads ancestors' required properties, private ones included
  (`own_required_properties`, `crates/nvs-types/src/signatures.rs:2830`; `ctor_init.rs:479`).
- Finality and redeclaration checks in a subclass read the parent's private members
  (`crates/nvs-types/src/conformance.rs:685`, `:80`, `:165`).

`property<T>` and `propertyPath<C>` read public names only (`retrieval.rs:732`). A check found later that
reads a private member from another file makes that member's row reach that file, and the surface module
doc lists it.

**There is no exhaustiveness check over an enum's cases** (`crates/nvs-types/src/expr/mod.rs:927`), so
an enum case is a member like any other. A case's value is inlined at each use (`ExprInfo::EnumCase`,
`crates/nvs-types/src/expr_table.rs:1081`), and so is a class constant's (`ExprInfo::ClassConst`,
`:1122`), so a changed value reaches the files that used it. If an exhaustiveness check is added later,
an enum's set of cases becomes part of its header row.

**`#[Core\Deprecated]` stays under "a member's attributes".** It changes `W1003` at every use
(`crates/nvs-types/src/deprecated.rs:108`), and every reader is a superset of the users.

**A failed lookup is a use.** The checker records the name it looked for and did not find: a class name,
`Class::member` when the receiver's type is known, and `::member` when it is not. `Indexed::unfollowed`
(`crates/nvs-lsp/src/index.rs:374`) already records `::name` for an unresolved call. The new record covers
every lookup kind. Without it, a new member could not reach the file that showed "unknown method".

**The unit of re-checking is the file.** A file whose uses meet the diff has all its bodies checked again.
A file's walk is one synthesized frame, and the anonymous-function count threads from file to file
(`crates/nvs-types/src/check.rs:316`, `:489`), so a body-level key would need a second numbering. A
reached file is one file's check, and the probe measures milliseconds for that. **Declined:** a key per
body. Reopen only if stage 5's measurement shows one file's re-check over 20 ms on the 972-file tree.

**A checked output's key is**: its own text hash, the row hashes of every symbol it used (sorted), the
presence of every name it failed to find, the header rows of its supertypes, the extension digest, the
restrictions digest, the configuration, and its anonymous-function start. The graph key leaves the
per-file key. Whole-graph passes (`after_walk`, signatures, the tables) still run on every analysis, as
now (`crates/nvs-types/src/check.rs:285`).

**An edit queues the entries that own a reached file**, not every reader. An entry that owns no reached
file keeps its records. Its files' text did not change, and nothing they used changed either. The
existing rule stays where the table says "every reader". The open documents among the readers are still
analysed at once, as now.

**The fresh-analysis oracle is the safety net.**
`a_cached_analysis_reports_what_a_fresh_one_does_after_every_kind_of_edit`
(`crates/nvs-lsp/tests/incremental.rs:693`) gains one edit per row of the table, each at every visibility.
A disagreement is a bug in the reach, never in the oracle.

**What it spends**: one interned symbol id per distinct use per file, and one 32-byte hash per
declaration. That is O(project) and does not grow with edits. The uses go into `Indexed` and the index
file, so `FORMAT` (`crates/nvs-lsp/src/disk_index.rs:76`) is raised once, in stage 4. The memory
measured by `bun nv lsp-start` before and after goes into the `session.rs` module doc.

**The counted tests live in a new file, `crates/nvs-lsp/tests/reach.rs`.** It generates its tree as
`tests/incremental.rs` does: one library `lib/Framework/` with a base class, a subclass, an interface,
an implementer, an enum and a class with private, protected and public members, and 200 entry points.
Each test makes one edit and asserts the exact set of files checked (`Work::checked` and the checked
paths) and the exact pending set. Neutral names only.

**Lanes.** Every stage edits `crates/nvs-lsp/src/` or the checker that stage 3 builds on, so the stages
run in order. Stage 1 and stage 2 touch disjoint crates and may run as side goals at the same time.

## Stages

### 1. one hash per declaration, and the diff of two parses

**Does:** splits the surface into rows keyed by symbol, and diffs two parses into the symbols that were
added, removed or changed, each with its visibility and kind.

**Split:** none, one session.

- [x] `crates/nvs-lsp/src/surface.rs:1`: `rows(stmts, trivia, text) -> Rows`, one row per declaration as
  the standing decision says, plus the class header rows and the graph row. `of` becomes the hash of all
  rows. The module doc describes rows, and the whitespace text from `1117cd5bbc` stays.
- [x] `crates/nvs-lsp/src/surface.rs`: `diff(old: &Rows, new: &Rows) -> Diff`, giving each added,
  removed or changed symbol with its visibility and its kind (member, header, enum cases, attributes,
  graph).
- [x] `crates/nvs-lsp/src/parse_cache.rs:331`: the cache entry keeps the `Rows` beside the hash.
  `hashes` (`parse_cache.rs:296`) gives them too.
- [x] Unit tests in `crates/nvs-lsp/src/surface.rs`: one per row of the standing decision's table, each
  asserting the diff's symbol, visibility and kind. Also: a whitespace or comment edit gives an empty diff,
  and a property's visibility, a property's type and a constant's value are each in their row.

### 2. the checker records what each file used and failed to find

**Does:** fills a per-file uses set and a per-file missed set while the bodies and declarations are
checked.

**Split:** none, one session. The sites are listed below. If the session nears 120k, the `nvs-hir`
member resolver item is the one to hand to the next session.

- [ ] `crates/nvs-types/src/partial.rs:61` `FileOutput`: `uses` and `missed`, sets of interned symbols.
  The symbol interner lives beside the type interner in `crates/nvs-lsp/src/session.rs`, and `nvs check`
  passes none, so it pays nothing.
- [ ] Type names: `lower_type` (`crates/nvs-types/src/lower.rs:1195`, failure `:1228`) records the class
  header symbol or the missed name. So do `new` (`expr/calls.rs:2976`), static and constant receivers
  (`expr/members.rs:1911`, `:2947`) and attribute names (`attributes.rs:469`, `:493`).
- [ ] Members: each wrapper in `crates/nvs-types/src/signatures.rs` records the declaring owner's
  member. On failure it records `Class::member` for the receiver's class. The wrappers: `resolve_method` (`:2682`),
  `resolve_method_ref` (`:2695`), `resolve_property` (`:2222`), `resolve_property_owned` (`:2242`),
  `resolve_const` (`:2283`), `resolve_const_owned` (`:2300`). A failure with no known receiver records
  `::member`, as `UnresolvedCall` (`expr_table.rs:1654`) does now. Enum cases at `expr/members.rs:125`.
- [ ] The `nvs-hir` member resolver records the same way: `MemberResolver::check`
  (`crates/nvs-hir/src/members.rs:412`), `member_declared` (`:2240`), failure `E_UNDEFINED_MEMBER`
  (`:2120`), and the missing enum member (`:2101`).
- [ ] Supertypes: `resolve_supertype` (`crates/nvs-hir/src/hierarchy.rs:827`) records the parent's and
  each interface's header symbol into the subclass's file, and its failure (`:898`) the missed name.
- [ ] Doc tags: `check_throws` and `check_see` (`crates/nvs-hir/src/members.rs:1022`, `:1223`) record the
  names they resolve or miss.
- [ ] Codec sites: `classes_reached` (`crates/nvs-types/src/derive.rs:2367`) puts every class a site
  reaches into the site file's uses, as a header use plus each property, private ones included.
- [ ] Rust tests in `crates/nvs-types/src/partial.rs`: a fixture file's `uses` and `missed` are exactly the
  expected sets. One test each: a resolved call, an unknown class, an unknown member on a known type, an
  unknown member on an unknown type. Then a type annotation, an enum case, a subclass and a
  `decodeAs<T>`.

### 3. a file's checked output is keyed on what it used

**Does:** replaces the per-graph key with the per-file key of the standing decision, so a changed
declaration re-checks only the files that used it.

**Split:** none, one session.

- [ ] `crates/nvs-lsp/src/session.rs:148` `Session::keys`: the per-file key the standing decision
  gives. The module doc's "a finer key is not built" becomes the new key's description.
- [ ] `crates/nvs-lsp/src/document.rs:1442` `graph_key`: no longer part of any file's key. What still
  reads it keeps it, or it is deleted.
- [ ] `crates/nvs-lsp/tests/incremental.rs:693`: the oracle gains one edit per row of the table, each at
  public, protected and private. The check `oracle` runs it.
- [ ] `crates/nvs-lsp/tests/incremental.rs`: the existing counted tests that assert a surface edit checks
  every file are updated to the new counts. Each says in its doc which row of the table it pins.

### 4. an edit queues only the entries that own a file it reaches

**Does:** makes `SymbolIndex::refresh` queue the owners of the reached files, persists the uses, and adds
the counted tests on a 200-entry tree.

**Split:** the tree generator and the eleven tests of `crates/nvs-lsp/tests/reach.rs` go to one writing
subagent once `refresh` lands. The session builds once it ends.

- [ ] `crates/nvs-lsp/src/index.rs:807` `refresh`: takes the diff of the old and new rows. From the
  uses, missed names and subtypes it works out the reached files. It queues the entry that owns each.
  `readers_of` (`index.rs:1376`) is kept for the rows marked "every reader".
- [ ] `crates/nvs-lsp/src/index.rs:360` `Indexed`: carries the file's uses and missed names. The index
  keeps postings from a symbol to the files that use or missed it, so a lookup is not a scan.
- [ ] `crates/nvs-lsp/src/disk_index.rs:76`: `FORMAT` raised, and the uses written and read.
- [ ] `crates/nvs-lsp/src/index.rs:664`: the start's load compares rows, not one hash, for a file that
  changed on disk, and queues as `refresh` does.
- [ ] `crates/nvs-lsp/tests/reach.rs:1`: the generated tree and the eleven tests the check `reach` names.

### 5. the rule, the measurement and the change entry

**Does:** writes down the new behaviour, measures it on the 972-file tree, and tells the user.

**Split:** none, one session.

- [ ] `docs/rules/ide/the-index-is-incremental-and-persistent.md:44`: the surface bullet describes rows
  and the reach table. The check-output bullet gives the per-file key. The Declined entry "A per-file
  dependency set in the check output's key" is removed, and "a key per body" is added with its reason.
  `bun nv rules --render`.
- [ ] `tools/nv/cmd/lsp-start.ts:1`: `--edits` takes a kind (`member`, `private`, `public`, `comment`)
  so each case's `checked` count can be measured. The before and after counts for each kind on
  `--copies 12` go into the rule's measured line.
- [ ] The memory figures before and after go into the `crates/nvs-lsp/src/session.rs` module doc.
- [ ] `website/src/data/changes.json`: one `changed` entry for `tools:editor`: "An edit in the editor
  checks again only the files that use what changed." `website/src/content/docs/tools/editor/index.mdx`
  is reread and stamped (`bun nv site --stamp`). No other page or `about.md` changes.

### 6. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of the change entry ([prose.md](../agent/prose.md) § *A newcomer reads it*).
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stage 1 is done. `crates/nvs-lsp/src/surface.rs` cuts a parse into `Rows` (one `Row` per symbol and
`Kind`: `Graph`, `Header`, `Cases`, `Properties`, `Method`, `AbstractMethod`, `Property`, `Constant`,
`Case`), each with its visibility, a text hash and an attributes hash. `of` is the hash of all rows, so
every caller of the old hash still works. `diff(&old, &new) -> Vec<Change>` gives symbol, kind (or
`Attributes`), the wider visibility and `How` (`Added`, `Removed`, `Changed`). A cut is written as a
gap, so adding an attribute or a member leaves the other rows as they were. The parse cache keeps an
`Arc<Rows>` per entry, and `ParseCache::hashes` returns `(Digest, &Rows)`. Thirteen unit tests at the
end of `surface.rs` (`cargo test --lib surface::tests`) and the five surface-hash tests in
`tests/incremental.rs` pass. No test is expected to go red. The two new rows and the constant-reads-a-
constant case are recorded under *Standing decisions*.

### Next

Stage 2, the checker records what each file used and failed to find (`crates/nvs-types/src/partial.rs`,
`check.rs`, `signatures.rs`, `crates/nvs-hir/src/members.rs`). It does not touch `nvs-lsp`.

- [ ] `crates/nvs-types/src/partial.rs:61` `FileOutput`: `uses` and `missed`.
- [ ] Type names at `crates/nvs-types/src/lower.rs:1195`, members in `signatures.rs` (wrappers at
  `:2222`, `:2242`, `:2283`, `:2300`, `:2682`, `:2695`), and a constant's value read through another
  constant (*Standing decisions*).
- [ ] `crates/nvs-hir/src/members.rs:412` and `crates/nvs-hir/src/hierarchy.rs:827`.
- [ ] Codec sites at `crates/nvs-types/src/derive.rs:2367`, and the Rust tests in `partial.rs`.

### Backlog
