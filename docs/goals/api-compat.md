---
looper: 1
title: "`nvs` extracts a package's public API and finds every break between two versions"
summary: "nvs pkg api writes a package's public API from the checked program without running any of it, nvs pkg diff classifies every change between two versions and exits non-zero on a break that the version step does not allow, the registry and the website have one JSON form to call, the commands have their feature proofs, rule packaging/only-a-major-may-break-the-api is shipped, and the checks below pass."
context: [crates/nvs-cli/src/api_diff.rs, crates/nvs-cli/src/main.rs, docs/rules/packaging/only-a-major-may-break-the-api.md]
stages:
  - id: 1
    title: "`nvs pkg api` writes a package's public API without running it"
    status: done
    context: [crates/nvs-hir/src/members.rs, crates/nvs-hir/src/hierarchy.rs, crates/nvs-hir/src/symbol.rs, crates/nvs-cli/src/package/mod.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api, packaging/nothing-runs-before-the-program, packaging/package-toml-is-a-tool-input]
  - id: 2
    title: a removal, and an addition that breaks a caller, are breaks
    status: done
    context: [crates/nvs-cli/src/api_diff.rs, crates/nvs-cli/src/package/diff.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api]
  - id: 3
    title: a parameter that narrows and a return that widens are breaks
    status: done
    context: [crates/nvs-cli/src/package/diff.rs, crates/nvs-types/src/ty.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api]
  - id: 4
    title: the version step decides, and the commands have their feature proofs
    status: done
    context: [docs/reference/tools/35-packages.md, docs/rules/packaging/below-1-0-the-breaking-slot-moves-left.md, crates/nvs-cli/src/package/manifest.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api, packaging/below-1-0-the-breaking-slot-moves-left]
  - id: 5
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: compat-proofs
    stage: 4
    name: the API commands owe no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:packages/nvs-pkg-api-and-nvs-pkg-diff"]
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, api-compat]
    reach: true
    floor: false
---

# `nvs` extracts a package's public API and finds every break between two versions

## Why

A package's version is a promise: from `0.1.0` on, a minor or patch release does not break the public
API of the release before it. A promise nobody checks is a guess, so `nvs` checks it statically at
publish, and the registry refuses a release that breaks it
(`rule:packaging/only-a-major-may-break-the-api`). This goal builds the extraction and the comparison
as `nvs` subcommands. The registry service (goal `registry-service`) and its website (goal
`registry-website`) call the same commands, so the publisher, the registry and the reader see one
answer. It runs before the client on the chain because every publish depends on it, and it needs
nothing from the client.

## Standing decisions

**Versions are `MAJOR.MINOR.PATCH`, read literally.** `0.0.x` is experimental and never checked. From
the first `0.1.0` or higher, the API is stable: a minor or patch release may add and widen, and may not
break. A major may change anything. This is not the toolchain's own scheme
(`rule:packaging/below-1-0-the-breaking-slot-moves-left`), and the new rule says so in one sentence.

**A step the versions cannot name allows no break.** A side with no version (no `package.toml`, no
`version` key, or a document with no `--from`/`--to`) allows no break, and so does a new version that
is the old one or lower. A version is three plain decimal numbers with no leading zero and nothing
after the third, so `1.2.3-beta` is an error naming the file. `0.0.9` to `0.1.0` is experimental
because a side is `0.0.x`: the first stable release freezes whatever it ships. The JSON form adds
`from`, `to`, `step` and `allowed` beside `changes`, at the same schema `1`, since nothing reads it
yet. Why: the standing decision errs towards breaking, and the registry, which always knows both
versions, reads one `allowed` rather than redoing the step. `crates/nvs-cli/src/package/diff.rs`'s
§ *The version step* is the home.

**The break list is the brief's, and it errs towards breaking**, as `nvs api diff` does for OpenAPI
(`crates/nvs-cli/src/api_diff.rs:24`). A break is: removing or renaming anything public, a new method
on a public interface, a new abstract method, a new enum case (it breaks an exhaustive `match`), a
narrowed parameter, a widened return and a new required parameter. Anything the comparison cannot
decide is a break. There is no suppression mechanism.

**The guarantee is about API shape only.** Behaviour may change in any release, and the reference
chapter says so.

**The API is extracted from the checked program, never from text and never by running it.** The
extraction reads declarations after name resolution, so an alias, an import or a formatting change is
not a change. A type is written in its resolved, normalised form, and two versions compare a type by
its fully qualified name. Stage 3 classifies types over the two extracted documents alone: union
membership, `null`, and a class against the parents each side's document lists. It does not call the
checker's `is_assignable`, which works inside one program. Why: one comparison over data the registry
also holds, and what it cannot read is a break.

**The commands are `nvs pkg api <dir>` and `nvs pkg diff <old> <new>`**, in a new `nvs pkg` group, as
`rule:packaging/package-toml-is-a-tool-input` spells every package verb. The existing OpenAPI command
is the top-level `nvs api diff` (`crates/nvs-cli/src/main.rs:380`, `crates/nvs-cli/src/api_diff.rs:69`).
It is a different group, so no word is shared, and `nvs pkg diff` never reads OpenAPI. `api` prints the
API as JSON with a schema version. `diff` takes two directories or two such JSON files, prints one line
per change with its class, has a `--json` form, and exits non-zero on a break the version step does
not allow. The version step is read from each side's `package.toml` `version`, or given with `--from`
and `--to`.

**This goal owns the `nvs pkg` group and `crates/nvs-cli/src/package/`.** Stage 1 creates
`package/mod.rs` with the group's `Args`, and `Command::Pkg` beside `Command::Api` in `main.rs`. Stage
4 creates `package/manifest.rs`, which reads `version` alone and leaves every other key to goal
`package-client`, whose stage 1 grows the same file into the full reader. Why: one module per command
group, and the goal that runs first on the chain creates it.

**Fixtures are package directory pairs written by the tests' author**, one `old/` and `new/` per
change class, under `crates/nvs-cli/tests/fixtures/package-api/<class>/`, each a few lines of Novis.
Why not `fixtures/api/`: `crates/nvs-cli/tests/openapi.rs:1204` walks that directory as OpenAPI
fixtures.

**A package's source is its `src/` directory, compiled through an entry that is never on disk.**
`nvs pkg api <dir>` requires every `.nvs` file under `<dir>/src` in path order, with the entry served
from the source map (`crates/nvs-cli/src/package/api.rs:73`), checks it, and runs nothing. Files
outside `src/`, such as tests, are not the API. A package that depends on another package does not
check yet, because nothing fetches its dependencies; goal `package-client` brings `vendor/`.

**The document is shape only, and every list but the parameters is sorted.** Each declaration has
`kind` (`class`, `interface`, `enum`, `type`), and a class, interface or enum always has
`constants`, `properties` and `methods`, empty or not. A member carries `visibility`, a property
also `set` (who may write it), and a parameter `name`, `type`, `optional`, `variadic` and `inout`.
A type is resolved and normalised: names fully qualified, an alias expanded, `self` and `parent`
replaced, `?T` written `T|null`, union and intersection members sorted. Doc comments, defaults'
values, constants' values and enum case values are left out, as behaviour. A class lists only the
`extends` and `implements` it writes; stage 3 closes over the parents in the document. Why: a
uniform shape keeps the stage 2 comparison one walk with no absent-key cases.

**`nvs pkg diff` adds three breaks the brief did not list, all towards breaking.** A new
optional parameter breaks on an interface's method and on an abstract one, because every class
that writes the method must add it; on any other method it is additive, as the brief says. A
renamed parameter breaks, because a call may name it. A key the walk does not know whose value
differs breaks, so a newer document is never compared as if its new keys were absent. Why: the
standing decision errs towards breaking, and each of these breaks a caller or an implementing
class that compiled before.

**A type is covered only when it surely is, and in both documents.** `mixed` covers anything,
`never` is covered by anything, `bool` covers `true` and `false`, and a class is covered by its
closed parents. `float` does not cover `int`, and a generic argument, a shape, a signature or a
class with an undeclared ancestor is undecidable, so a break. A class's ancestry must hold in the
old and the new document, since a changed parent is itself reported on that class. Why: the
standing decision errs towards breaking, and a wrongly additive line is the one error that lets a
break through. The module doc of `crates/nvs-cli/src/package/diff.rs` § *Types* is the home.

**Trade-offs.** No program changes, and nothing runs on the request path. A publish pays one compile
of each version. A package author loses the freedom to break a stable API in a minor, which is the
point. Simplicity: one check the user already expects from semantic versioning, applied by a tool
rather than by convention.

**The attack is a program, and the version step's attack is a Rust test.** A hostile case is a
`.nvs` file the runner runs, so it cannot call `nvs pkg diff`. The attack writes one type many ways,
and `a_type_written_many_ways_has_one_form` extracts it as a package and pins each type's one form.
A minor that removes a method under a `--from`/`--to` or `package.toml` that tries to hide it is
`the_version_step_decides_whether_a_break_fails` and `each_side_reads_its_version_from_package_toml`.
Why: each claim is held by something that runs it, and a comment that only describes a command is
not an attack.

**The two OpenAPI emitter gaps moved to goal `first-packages`.** `oauth2-scopes-per-operation` and
`two-routes-that-write-one-template` named this goal but are about `crates/nvs-cli/src/openapi.rs`,
which this goal never touches. `first-packages`' stage 1 already edits that emitter and owns its
`info-version` gap, and `Novis\Rest` reads OAuth2 scopes.

**Lanes.** Every stage touches `crates/nvs-cli/src/package/`, so the stages run in order. Stage 1 edits
the `Command` enum in `crates/nvs-cli/src/main.rs`, which goals `package-client` and `registry-log` also
edit; stage 4 adds a heading to `docs/reference/tools/35-packages.md`, which goal `namespace-grants`
creates and goal `package-client` also edits. Those merge one after another. The goal is not a side
goal: it runs in its chain place.

## Stages

### 1. `nvs pkg api` writes a package's public API without running it

**Does:** the public classes, interfaces, enums, functions and constants of a package, with every
public and protected member's signature, are written as deterministic JSON.

- [x] The `nvs pkg` group: `crates/nvs-cli/src/package/mod.rs` with its `Args`, and `Command::Pkg` beside `Command::Api` (`rule:packaging/package-toml-is-a-tool-input`). `crates/nvs-cli/src/main.rs:380`, `crates/nvs-cli/src/package/mod.rs:1`
- [x] The extraction over resolved declarations: visibility, `abstract`, `final`, `readonly`, enum cases, parameters with their types, defaults present or not, return types, and each class's parents. `crates/nvs-cli/src/package/api.rs:1`, `crates/nvs-hir/src/members.rs:136`, `crates/nvs-hir/src/hierarchy.rs:222`
- [x] The JSON with a schema version, sorted so it is byte-identical across runs and platforms. `crates/nvs-cli/src/package/api.rs:1`
- [x] Tests: the JSON of a fixture package is pinned byte for byte, and a package with an effectful top-level statement produces no effect (`rule:packaging/nothing-runs-before-the-program`). `crates/nvs-cli/tests/package_api.rs:1`

### 2. a removal, and an addition that breaks a caller, are breaks

**Does:** `diff` classifies every declaration and member change as breaking or additive.

**Split:** after the classifier lands, one writing subagent writes the fixture pairs of stages 2 and 3,
one directory per class in the standing decision's break list and each additive class.

- [x] `nvs pkg diff <old> <new>` over two directories or two JSON files, one line per change, and `--json`. `crates/nvs-cli/src/package/diff.rs:1`, `crates/nvs-cli/src/package/mod.rs:1`
- [x] A removed or renamed public declaration or member, and a lowered visibility, are breaks (`rule:packaging/only-a-major-may-break-the-api`). `crates/nvs-cli/src/package/diff.rs:1`
- [x] A new method on a public interface, a new abstract method, a new enum case and a new required parameter are breaks. `crates/nvs-cli/src/package/diff.rs:1`
- [x] A new class, a new concrete method and a new optional parameter are additive. `crates/nvs-cli/src/package/diff.rs:1`
- [x] The fixture pairs, and a test that each gives its class. `crates/nvs-cli/tests/fixtures/package-api/removed-method/old/src/Api.nvs:1`, `crates/nvs-cli/tests/package_api.rs:1`

### 3. a parameter that narrows and a return that widens are breaks

**Does:** a type that changes direction is classified by which side of the call it is on.

- [x] Parameter types may only widen and return types may only narrow; a union that gains a member, a nullable that appears and a class replaced by its parent are each classified by the side, over the two documents as the standing decision says. `crates/nvs-cli/src/package/diff.rs:1`, `crates/nvs-types/src/ty.rs:29`
- [x] A change the comparison cannot decide is a break, and the line says so. `crates/nvs-cli/src/package/diff.rs:1`
- [x] Tests: the type fixture pairs, one per direction and side, and one undecidable pair. `crates/nvs-cli/tests/package_api.rs:1`

### 4. the version step decides, and the commands have their feature proofs

**Does:** `diff` exits non-zero only on a break the version step forbids, and both commands have their
proofs.

**Split:** after the version step lands, one writing subagent writes the about page and the examples,
then a second the attack, each held to § *Text an end user reads* in `AGENTS.md`.

- [x] `package/manifest.rs` reads `version` from each side's `package.toml`, and `--from` and `--to` replace it. `crates/nvs-cli/src/package/manifest.rs:1`
- [x] `0.0.x` on either side is never checked, a major allows anything, and a minor or patch allows only additions; a test for each. `crates/nvs-cli/src/package/diff.rs:1`, `crates/nvs-cli/tests/package_api.rs:1`
- [x] The heading `# nvs pkg api and nvs pkg diff` in `docs/reference/tools/35-packages.md`, which says the guarantee is about shape only. `docs/reference/tools/35-packages.md:124`
- [x] The about page and the examples. `docs/examples/tools/packages/nvs-pkg-api-and-nvs-pkg-diff/about.md:1`
- [x] The attack: one type written many ways, which the extraction writes in one form; a break in a minor that `--from`/`--to` tries to hide is `the_version_step_decides_whether_a_break_fails`. `tests/hostile/tools/packages/nvs-pkg-api-and-nvs-pkg-diff/01-one-type-written-many-ways.nvs:1`
- [x] `bun nv proofs --id 'tools:packages/nvs-pkg-api-and-nvs-pkg-diff'` owes nothing. `docs/reference/tools/35-packages.md:124`
- [x] `rule:packaging/only-a-major-may-break-the-api` is `shipped` with its cases in `guardedBy`, and `bun nv owners --closes api-compat` passes. `data/rules/packaging/only-a-major-may-break-the-api.json:3`

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1 to 4 are done. Stage 4's last group landed the reference heading `# nvs pkg api and nvs
pkg diff` at the end of `docs/reference/tools/35-packages.md` (its `<!-- src: -->` follows the
section, as `25-server.md` does), the about page, three examples with `.out` files, and the attack
`01-one-type-written-many-ways.nvs`. Four tests in `crates/nvs-cli/tests/package_api.rs` carry
`// covers:`, one of them new: `a_type_written_many_ways_has_one_form`, which copies the attack into
a temporary package and pins each type's form; it passed, run once. The rule's JSON is `shipped`
with `guardedBy`. The two OpenAPI gaps this goal held moved to `first-packages` (standing decision).
`website/public/llms-full.txt` is rendered. The proofs were small, so no subagent wrote them.

Green this session: `bun nv proofs --gate --only tools:packages/nvs-pkg-api-and-nvs-pkg-diff`,
`bun nv proofs --run --id …` (3 examples, 1 attack), `bun nv proofs --comments` over both trees,
`bun nv owners --closes api-compat`, `bun nv owners --check`, `bun nv rules --check`, `looper lint`,
clippy with `-D warnings`, rustfmt on the test file.

The acceptance sweep's reds were already fixed on disk: `env-secret-proofs`'s log shows exit 0, and
the nightly run 40474302703 is on `55b03e97a`, before `22f1e4ec4`, `e461612ce` and `c33a8b8f7`,
which fix its three Miri jobs.

Tests that may go red at the gate: `COMMAND_NAMES`' table test in `crates/nvs-cli/src/main.rs`
(30 entries, not run), any proofs or help-text check that counts subcommands or pins `nvs pkg diff
--help`, the reference leg if it reads the new chapter's `text` fence, and `bun nv ci --check`'s
website render for the changed chapter title.

### Next

**Stage 5, the gate.** Start `looper verify` with `run_in_background`, write nothing else, and end
the turn. Group its failures by cause and fix them in one pass, then run it once more. Before
`DONE`: `bun nv verify --doc`, `bun nv owners --closes api-compat` and `bun nv ci --check`, each red
fixed. Then move each decision only this file's prose holds into `diff.rs`'s module doc or the rule,
and delete the goal file.

- [ ] `looper verify` once over the whole goal, failures grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure fixed in one pass, and the second run green. `tools/nv/cmd/verify.ts:1`
- [ ] The goal-end gates: `bun nv verify --doc`, `bun nv owners --closes api-compat`, `bun nv ci --check`. `docs/goals/api-compat.md:1`

### Backlog

- The registry calls `nvs pkg diff` at publish in goal `registry-service` (`docs/goals/registry-service.md`).
- The website shows the API diff per version in goal `registry-website` (`docs/goals/registry-website.md`).
