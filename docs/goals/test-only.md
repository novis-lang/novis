---
looper: 1
title: a declaration marked TestOnly is test code that only tests can use or see
summary: "Reached when #[TestOnly] on a class, interface, enum or method makes it test code: nvs run and nvs build leave it out, a use outside test code does not compile, Core\\Program::implementing never lists it, the editor offers it only inside test code, and the rules, the feature proofs and the website say so."
context: [docs/rules/testing/tests-never-reach-a-build.md, docs/rules/testing/private-in-the-same-file.md, crates/nvs-types/src/test_code.rs, crates/nvs-types/src/derive.rs]
stages:
  - id: 1
    title: "#[TestOnly] exists, and what it marks joins the test-code table"
    status: done
    context: [docs/rules/attributes/an-attribute-is-declared-with-its-sites.md, crates/nvs-types/src/check.rs, crates/nvs-diagnostics/src/code/types_e10.rs]
  - id: 2
    title: "every use of test-only code outside test code does not compile"
    status: done
    context: [crates/nvs-types/src/expr_table.rs, crates/nvs-types/src/deprecated.rs, crates/nvs-diagnostics/src/code/types_e08.rs]
  - id: 3
    title: "a run and a build leave every test-only type out, and Core\\Program never lists one"
    status: done
    context: [docs/rules/programs/implementing.md, crates/nvs-ir/src/lower/mod.rs, crates/nvs-hir/src/hierarchy.rs, crates/nvs-ir/src/lower/expr.rs]
  - id: 4
    title: "the editor offers test-only code only inside test code, and its hover says it is for tests"
    status: done
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/hover.rs]
  - id: 5
    title: "the rules, the reference, the examples, the feature proofs and the website say so"
    status: open
    context: [docs/reference/lang/95-testing.md, docs/examples/lang/testing/chapter.md, website/src/data/changes.json]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: marker
    stage: 1
    name: "#[TestOnly] makes a method, and every method of a class, interface or enum, test code, and a wrong place for it does not compile"
    run: [cargo, test, --test, test_only]
    tests: [a_test_only_method_is_test_code, every_method_of_a_test_only_type_is_test_code, test_only_on_a_property_or_constant_does_not_compile, a_test_only_method_cannot_override_production_code, an_override_of_a_test_only_method_must_be_test_only, production_code_cannot_extend_or_implement_a_test_only_type]
    timeout: 1800
  - id: uses
    stage: 2
    name: "a call, a new, a static member, a constant and a type written outside test code that name test-only code do not compile, and test code uses it freely"
    run: [cargo, test, --test, test_only]
    tests: [a_call_to_a_test_only_method_outside_test_code_does_not_compile, new_of_a_test_only_class_outside_test_code_does_not_compile, a_static_member_of_a_test_only_type_outside_test_code_does_not_compile, a_test_only_type_in_a_production_signature_does_not_compile, test_code_and_test_only_code_use_test_only_code]
    timeout: 1800
  - id: builds
    stage: 3
    name: "a run lowers no test-only code and has no class row for a test-only type, Core\\Program::implementing lists none, and a package's test-only code reaches the application's tests"
    run: [cargo, test, --test, test_only_run]
    tests: [a_run_cannot_find_a_test_only_class_by_name, implementing_never_lists_a_test_only_class, a_package_test_only_class_is_usable_from_the_application_tests, nvs_test_runs_a_test_that_uses_test_only_code]
    timeout: 1800
  - id: editor
    stage: 4
    name: "completion offers test-only code only inside test code, and a hover on it says it is for tests"
    run: [cargo, test, --test, test_only_editor]
    tests: [completion_outside_test_code_offers_no_test_only_member, completion_outside_test_code_offers_no_test_only_class, completion_inside_test_code_offers_test_only_code, a_hover_on_test_only_code_says_it_is_for_tests]
    timeout: 1800
  - id: proofs
    stage: 5
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, test-only]
    timeout: 1800
  - id: close
    stage: 6
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, test-only]
    timeout: 900
---

# a declaration marked TestOnly is test code that only tests can use or see

## Why

A project writes helpers so its tests can reach what they need: a fake clock, a factory, a method
that empties a cache, a framework's sign-in shortcut for tests. Today each one is ordinary code. It
is public, it ships in every build, and the editor offers it everywhere. Only its name and its doc
comment tell a developer not to call it, and a request can reach it in production.

Novis already has test code (`rule:testing/tests-never-reach-a-build`). A method with one of the
seven testing markers is never lowered by `nvs run` or `nvs build`, and a call into it from other code
is `E0889`. Only those markers count, so a helper has no way to join. This goal adds one attribute,
`#[TestOnly]`, that makes a declaration test code. Everything the rule already does then holds for it.

The user raised it on 2026-11-26 for code sanity in big projects. The stronger reason is the first
priority, security and request isolation: a test seam left in a build is attack surface, and a
test-only declaration is not in the build at all.

## Standing decisions

**The attribute.** `#[TestOnly]`, resolved nominally as `Core\TestOnly`, with no fields. It is not
under `Core\Test\*` with the testing markers, because it is written in production files, beside the
code it marks, and `#[TestOnly]` reads as a plain statement there. Its row joins `ATTRIBUTE_DOCS`
(`crates/nvs-types/src/derive.rs:221`) with the sites `Class`, `Interface`, `Enum` and `Method`, as
`DEPRECATED`'s row (`:755`) lists its sites. The row's `short` and `site` texts are text an end user
reads. Fallback if the name collides with a user class in the corpus: none needed, the match is
nominal (`rule:testing/test-attribute`'s last paragraph).

**What it makes test code.** A method that carries it, and every method of a class, interface or enum
that carries it, the constructor included. `crates/nvs-types/src/test_code.rs` `build_table` (`:60`)
adds both to the `TestCode` set beside the `MARKERS` (`:36`), and keeps a second set of test-only
types for the uses a method set cannot see (§ *What counts as a use*). A method that is test code
for any reason is test code everywhere the set is read: lowering, the method table, mutation
(`crates/nvs-ir/src/lower/mod.rs:1239`) and `private-in-the-same-file`. That last one means a
test-only helper in the same file as a class reaches its `private` members. That rule's text already
says a helper that reads a `private` member "has to be test code itself", so this is the rule
working, not a widening.

**Where it is not allowed**, each a new error `E1019` in `crates/nvs-diagnostics/src/code/types_e10.rs`
(the next free code; `types_e08.rs` ends at `E0899` and `E0901` is internal):
- On a property, a constant, an enum case or a parameter. The attribute table's site list already
  refuses these with the ordinary wrong-site error, so `E1019` is not needed for them. A test-only
  property would give an object a different layout under `nvs test` than under `nvs run`, and a
  codec or `Core\Reflect` would show tests a shape production never has.
- On a method of a type that is not test-only, when the method overrides or implements a method
  that is not test code. Production calls that method through the parent type, so it must exist in
  a build. A test-only type may implement a production interface, such as a fake `Clock`, because
  no object of it exists outside tests (decided in stage 1, so the fake-clock example compiles).
- On a method that is not test code and overrides or implements a test-only one. It must carry
  `#[TestOnly]` too, or production could call the name through the subclass.
- On a class or interface that is not test code and extends or implements a test-only type, and on a
  type that is not test code with a test-only type as a type argument in its `extends` or
  `implements`. The type-argument half is checked with stage 2's type positions, as `E0889`.
- A `#[TestOnly]` on a member of a type that already carries it is allowed and says nothing.

**What counts as a use.** Everything `[[restrict]]`'s use list names
(`rule:packaging/a-restriction-limits-where-a-declaration-is-used` § *What counts as a use*), and on
top of it every type position: a parameter type, a return type, a property type, `is`, `catch`,
`class<T>` and a type argument. A restriction lets allowed code pass a restricted object through typed
signatures. Here no production signature may name a test-only type, because the type has no class
row in a build (stage 3). A use outside test code is `E0889`. Its message becomes one shape for both
kinds: "`Cache::clearAll` is test code, so only test code can use it", with the primary label "this
code is not a test". The help names `#[TestOnly]` when the target carries it: "It is marked
`#[TestOnly]`. Call it from a method with a testing attribute or from other `#[TestOnly]` code."
`crates/nvs-types/src/test_code.rs` `refuse_calls` (`:97`) grows to read `New` (`expr_table.rs:612`),
`StaticProperty` (`:770`), `ClassConst` (`:1140`) and `ClassConstLate` (`:1170`) for the type set,
and the type positions are checked where a body's signature is resolved (`crates/nvs-types/src/check.rs:1480`
is where a method's test-code flag is read). With no `#[TestOnly]` in the program the type set is
empty and the pass returns at once, as `:103` does today.

**Inside test code.** A method with a testing marker, a test-only method and every method of a
test-only type may use test-only code. A test-only type may be named in another test-only type's
signatures. An anonymous function written inside test code is test code, as today.

**What a build leaves out.** A run and a build lower no test-only method (already true once it is in
the set, `crates/nvs-ir/src/lower/mod.rs:951`), and emit no class row, descriptor or layout for a
test-only type (the class rows are built around `mod.rs:1470`). So `Core\Reflect::forClass` with a
test-only class's name throws the error an unknown class throws (`crates/nvs-stdlib/src/reflect.rs:2788`),
and a dynamic call on a `mixed` receiver finds no test-only method, as for a test method today.
`nvs test` lowers all of it. `nvs check` checks all of it.

**`Core\Program::implementing`, `implementingWith` and `constructors` never list a test-only class,
in any build, `nvs test` included** (`crates/nvs-hir/src/hierarchy.rs:1175`, lowered at
`crates/nvs-ir/src/lower/expr.rs:4115` and `:4202`; the session finds where `constructors` expands).
A program then lists the same classes under `nvs test` as in production, so a test sees the module
set production runs. A test that needs a fake in the list passes it in by hand. Built (stage 3):
the filter is `crate::test_code::listed_implementors` in `nvs-types`, called by all three members
in `crates/nvs-types/src/program.rs`, which record the class list the lowering reads. The checker
already has the test-only set there, so `ClassLinks` gets no flag and `nvs_hir::implementors` stays
the plain `is` walk that the other enumerations (constructor checks, field scans) need.

**Packages.** A package's test-only code is test code wherever it is used, so the application's tests
may use a framework's test helpers. `rule:testing/test-attribute` drops a package's `#[Test]` rows,
not its test code, so nothing there changes. A package that adds `#[TestOnly]` to a declaration that
was not test code makes a breaking change (`rule:packaging/only-a-major-may-break-the-api`), and
`nvs pkg diff` reports it as it reports a removed declaration.

**The editor.** Outside test code, completion leaves test-only members and test-only types out
completely. A `[[restrict]]`-blocked item sorts last (`crates/nvs-lsp/src/completion.rs:5069`),
because a restriction's reason is worth reading. A test-only item outside test code is always an
error, so it is not offered. Inside test code it is offered as any member is, with no tag. "Inside
test code" is decided by the method around the cursor, which completion must read for this, unlike
`restricted_here` (`:5099`). A hover on test-only code adds one line, "Only for tests. `nvs run` and
`nvs build` leave it out.", chained beside the deprecation and restriction notes
(`crates/nvs-lsp/src/hover.rs:608`). Go to definition, find references and rename work as for any
declaration.

**Cost.** Nothing at run time, and a build gets smaller by every test-only declaration. At compile
time, one attribute check per declaration while `build_table` already walks them, one set of test-only
types, and one set lookup per recorded use and per type position when the set is not empty. Memory:
the two sets, O(test-only declarations), once per compile, nothing per request.

**Rules.** A new fragment `docs/rules/testing/a-test-only-declaration-is-test-code.md` holds the
attribute, its sites, the uses, the `E1019` places, `implementing`, packages, the editor, the cost
and § *Declined*. `tests-never-reach-a-build` names `#[TestOnly]` in its definition of test code, and
its Declined item "a test class is test code as a whole" gains one sentence: a class is test code
only when its author marks it, never because it has a `#[Test]` method. `private-in-the-same-file`'s
Declined `#[VisibleForTesting]` gains one sentence that `#[TestOnly]` narrows where that one would
widen. `docs/ground-rules.md` gets its line from `bun nv rules`.

**Declined, written into the new fragment's Declined list.**
- **A `[[restrict]]` entry with an `allow` of the test namespaces.** Test code is not a place: a
  test may live in any file. A restriction also keeps the code in the build.
- **Sorting a test-only item last in completion, like a restricted one.** It is always a compile
  error outside test code, so offering it only invites the error.
- **Test-only properties and constants.** They change the layout and the codec output between
  `nvs test` and `nvs run`, so a test would check a shape production never has.
- **Listing test-only classes in `implementing` under `nvs test` only.** The tests would then run a
  different module set from production.
- **Type positions allowed, as `[[restrict]]` allows them.** A test-only type has no class row in a
  build, and a production signature that names it is a test seam in the public API.
- **A warning instead of an error.** The code is not in the build, so the use cannot run.

**Feature proofs.** One new feature, `#[TestOnly]`. `bun nv proofs --owed --goal test-only` names it
and every feature the work reached. The three examples: a fake clock class that implements a
production `Clock` interface and is used by a test; a production cache with a `#[TestOnly]` method
that empties it between tests; a framework-like package whose test-only sign-in helper the
application's test calls. Each uses neutral names (`Shop`, `Framework`). The bench compiles a program
with many test-only declarations and states its growth, linear in them. The attack: production code
tries to reach test-only code by a direct call, a `mixed` receiver, `Core\Reflect::forClass` with the
class name as a string and `Core\Program::implementing`, and every attempt fails.

**Website.** One `website/src/data/changes.json` entry of kind `added` for `#[TestOnly]`, written to
AGENTS.md § *Text an end user reads*. The reference chapter `docs/reference/lang/95-testing.md` gets
a section after the hooks (`:1476`). The examples go under `docs/examples/lang/testing/` in a new
folder, and `chapter.md` lists it.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is written to `### Backlog` with what blocked it.

**Lanes.** It runs on the chain, after `refactor-commands` and `loadable-completion`, which also edit
`crates/nvs-lsp/src/completion.rs` (and `refactor-commands` `hover.rs`). Stages 1 to 3 touch only the checker, the lowering
and the hierarchy and could run beside `refactor-commands` as a side goal. Stage 4 runs after it.

## Stages

### 1. #[TestOnly] exists, and what it marks joins the test-code table

- [x] `crates/nvs-types/src/derive.rs:835`: a `TEST_ONLY` constant (`Core\TestOnly`) beside `TEST`,
  and its `ATTRIBUTE_DOCS` row (`:221`, shaped like `DEPRECATED`'s at `:755`) with the four sites and
  end-user `short` and `site` texts. `rule:attributes/an-attribute-is-declared-with-its-sites`.
- [x] `crates/nvs-types/src/test_code.rs:60`: `build_table` adds every `#[TestOnly]` method, and
  every method of a `#[TestOnly]` class, interface or enum, to the method set; a second set holds the
  test-only types. The module doc (`:1`) is rewritten whole to name the eighth way in.
- [x] `crates/nvs-diagnostics/src/code/types_e10.rs`: `E1019` with its card, and the checks of
  § *Where it is not allowed* where a class's parents and overrides are resolved
  (`crates/nvs-types/src/check.rs`; the session finds the override check by `E_` code).
- [x] `crates/nvs-types/tests/test_only.rs` (new): the six tests of the check `marker`, then
  `cargo test --test test_only` once.

### 2. every use of test-only code outside test code does not compile

- [x] `crates/nvs-types/src/test_code.rs:97`: `refuse_calls` also reads `New`, `StaticProperty`,
  `ClassConst` and `ClassConstLate` entries (`crates/nvs-types/src/expr_table.rs:612`, `:770`,
  `:1140`, `:1170`) against the type set, and the one message shape and help of § *What counts as a
  use*. `crates/nvs-diagnostics/src/code/types_e08.rs:1144`: `E0889`'s card says the same.
- [x] Type positions: a parameter, return or property type, `is`, `catch`, `class<T>` and a type
  argument outside test code that names a test-only type is `E0889`, checked where the signature is
  resolved (`crates/nvs-types/src/check.rs:1480` reads the method's test-code flag).
- [x] The five tests of the check `uses` in `crates/nvs-types/tests/test_only.rs`, then
  `cargo test --test test_only` once.

### 3. a run and a build leave every test-only type out, and Core\Program never lists one

- [x] `crates/nvs-ir/src/lower/mod.rs:1470`: under `Tests::Omit`, a test-only type gets no class row,
  descriptor or layout. Its methods are already left out once stage 1 put them in the set (`:951`).
- [x] `crates/nvs-hir/src/hierarchy.rs:1175`: the enumeration skips a test-only class in every build,
  through a flag on `ClassLinks` if the layer cannot see attributes. `crates/nvs-ir/src/lower/expr.rs:4115`
  and `:4202` and the `constructors` expansion read the same list. `rule:programs/implementing` gains
  one sentence.
- [x] `crates/nvs-ir/src/lower/tests.rs:6465`: `a_run_lowers_no_test_code_and_a_test_run_lowers_all_of_it`
  adds a test-only method and a test-only class to its program.
- [x] `crates/nvs-cli/tests/test_only_run.rs` (new): the four tests of the check `builds`, the
  package one with a fixture under `crates/nvs-cli/tests/fixtures/test-only/` laid out like
  `fixtures/package-api/restrict/`. Then `cargo test --test test_only_run` once.

### 4. the editor offers test-only code only inside test code, and its hover says it is for tests

Gate: `refactor-commands` and `loadable-completion` are reached (their goal files are gone from
`docs/goals/`). The test-only filter joins the loadable filter in `every_type` and `under`.

- [x] `crates/nvs-lsp/src/completion.rs:5069`: outside test code, a test-only member is left out of
  every member list (`members_of` `:676`, `member` `:3518`, `declared_members` `:4902`), and a
  test-only type out of the type-name list. The method around the cursor decides "inside test code".
- [x] `crates/nvs-lsp/src/hover.rs:608`: the "Only for tests" line beside `restriction` (`:644`) and
  `deprecation` (`:684`).
- [x] `crates/nvs-lsp/tests/test_only_editor.rs` (new, shaped like `deprecated.rs` beside it): the four
  tests of the check `editor`, then `cargo test --test test_only_editor` once.

### 5. the rules, the reference, the examples, the feature proofs and the website say so

**Split:** the three examples, the attack and the bench go to one writing subagent once the rule
fragment is written. Its brief names AGENTS.md § *Text an end user reads* and § *Feature proofs*
above.

- [ ] `docs/rules/testing/a-test-only-declaration-is-test-code.md` (new),
  `docs/rules/testing/tests-never-reach-a-build.md` and
  `docs/rules/testing/private-in-the-same-file.md`: per § *Rules*, then `bun nv rules`.
- [ ] `docs/reference/lang/95-testing.md:1476`: the new section after the hooks.
- [ ] The feature proofs `bun nv proofs --owed --goal test-only` names: about.md, three examples,
  the bench, the attack, the Novis and Rust tests and the help text. `docs/examples/lang/testing/chapter.md`
  lists the new folder.
- [ ] `website/src/data/changes.json:1`: the `added` entry, then `bun nv site --check stale`,
  `changes` and `review` green.
- [ ] `bun nv proofs --owed --goal test-only` names nothing.

### 6. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 4 are done. The checks `marker`, `uses` (`cargo test --test test_only`, 11 passed),
`builds` (`cargo test --test test_only_run`, 4 passed) and `editor`
(`cargo test --test test_only_editor`, 4 passed) are green, and the build and clippy are clean.

Stage 4: completion's `Cursor` carries `in_tests`, set by `test_code_at`: the cursor is inside a
test-only class, interface, enum or `extend` block, or inside a member span of a method the
`ExprTypeTable` calls test code. Outside it, `hidden` leaves out every member of a test-only type
and every method that is test code (a `#[Test]` method too, whose call is `E0889` as well), in
`own_members`, `block_rows` and `enum_cases`. `in_reach` and `under` leave out test-only types,
except in the qualified list a `use` line takes, because a test file imports its test-only types
at the top. Hover's `for_tests` adds "**Only for tests.** `nvs run` and `nvs build` leave it out."
between the restriction and the `///` run. Members of unloaded `extend` blocks have no checker
record and are not filtered.

Stage 3: `ExprTypeTable::is_test_only_type` answers the test-only types, `Tests::keeps_type` and
`Tests::lowers` leave them out of a run, and `listed_implementors` filters `Core\Program`.
`crates/nvs-cli/tests/test_only_run.rs` drives the binary over `fixtures/test-only/`.

Every job the nightly sweep named (run 45813627231, at `4e43e1fa2e`) was already fixed on `main`
after it: the Miri zone tests in `c115a735a1` and `215257ff46`, the Uuid `Parses` case in
`28a4d7a6bf` and the serve bound in `544711a8fb`. A sweep that names that run again needs nothing.

### Next

Stage 5 from its first item, following its `**Split:**` line. Write the rule fragment
`docs/rules/testing/a-test-only-declaration-is-test-code.md` from § *Rules* and § *Declined* in
the standing decisions, with a paragraph on the editor (`completion.rs` `hidden` and
`test_code_at`, `hover.rs` `for_tests`), and decide the backlog's route and command question there.
Then `bun nv rules`, the reference section at `docs/reference/lang/95-testing.md:1476`, and
`bun nv proofs --owed --goal test-only` for the list the proof subagent takes.

Tests expected to go red at the gate: any test that counts `ATTRIBUTES` or snapshots the attribute
cards, stubs or `nvs meta` output changes by one row, and any snapshot of `E0889`'s card or message
(the `nvs agent show` pages, `llms-full.txt`) changes with the new text. The new code `E1019` may
need a line wherever the error codes are listed for the website. A snapshot of
`ExprTypeTable`'s dump gains the empty `test_only_types` section.

### Backlog

- A `#[TestOnly]` class that declares routes or commands still reaches the compiled route and
  command tables, whose handlers then name a class a run has no row for. Stage 5's rule says whether
  that is `E1019`, and stage 6 adds the check if so (`crates/nvs-types/src/routes.rs`,
  `check_class_routes`, and `crates/nvs-types/src/commands.rs`).
