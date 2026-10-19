---
looper: 1
title: "`nvs` extracts a package's public API and finds every break between two versions"
summary: "nvs pkg api writes a package's public API from the checked program without running any of it, nvs pkg diff classifies every change between two versions and exits non-zero on a break that the version step does not allow, the registry and the website have one JSON form to call, the commands have their feature proofs, rule packaging/only-a-major-may-break-the-api is shipped, and the checks below pass."
context: [crates/nvs-cli/src/api_diff.rs, crates/nvs-cli/src/main.rs, docs/rules/packaging/only-a-major-may-break-the-api.md]
stages:
  - id: 1
    title: "`nvs pkg api` writes a package's public API without running it"
    status: open
    context: [crates/nvs-hir/src/members.rs, crates/nvs-hir/src/hierarchy.rs, crates/nvs-hir/src/symbol.rs, crates/nvs-cli/src/package/mod.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api, packaging/nothing-runs-before-the-program, packaging/package-toml-is-a-tool-input]
  - id: 2
    title: a removal, and an addition that breaks a caller, are breaks
    status: open
    context: [crates/nvs-cli/src/api_diff.rs, crates/nvs-cli/src/package/diff.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api]
  - id: 3
    title: a parameter that narrows and a return that widens are breaks
    status: open
    context: [crates/nvs-cli/src/package/diff.rs, crates/nvs-types/src/ty.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api]
  - id: 4
    title: the version step decides, and the commands have their feature proofs
    status: open
    context: [docs/reference/tools/35-packages.md, docs/rules/packaging/below-1-0-the-breaking-slot-moves-left.md, crates/nvs-cli/src/package/manifest.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api, packaging/below-1-0-the-breaking-slot-moves-left]
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
change class, under `crates/nvs-cli/tests/fixtures/api/<class>/`, each a few lines of Novis.

**Trade-offs.** No program changes, and nothing runs on the request path. A publish pays one compile
of each version. A package author loses the freedom to break a stable API in a minor, which is the
point. Simplicity: one check the user already expects from semantic versioning, applied by a tool
rather than by convention.

**Lanes.** Every stage touches `crates/nvs-cli/src/package/`, so the stages run in order. Stage 1 edits
the `Command` enum in `crates/nvs-cli/src/main.rs`, which goals `package-client` and `registry-log` also
edit; stage 4 adds a heading to `docs/reference/tools/35-packages.md`, which goal `namespace-grants`
creates and goal `package-client` also edits. Those merge one after another. The goal is not a side
goal: it runs in its chain place.

## Stages

### 1. `nvs pkg api` writes a package's public API without running it

**Does:** the public classes, interfaces, enums, functions and constants of a package, with every
public and protected member's signature, are written as deterministic JSON.

- [ ] The `nvs pkg` group: `crates/nvs-cli/src/package/mod.rs` with its `Args`, and `Command::Pkg` beside `Command::Api` (`rule:packaging/package-toml-is-a-tool-input`). `crates/nvs-cli/src/main.rs:380`, `crates/nvs-cli/src/package/mod.rs:1`
- [ ] The extraction over resolved declarations: visibility, `abstract`, `final`, `readonly`, enum cases, parameters with their types, defaults present or not, return types, and each class's parents. `crates/nvs-cli/src/package/api.rs:1`, `crates/nvs-hir/src/members.rs:136`, `crates/nvs-hir/src/hierarchy.rs:222`
- [ ] The JSON with a schema version, sorted so it is byte-identical across runs and platforms. `crates/nvs-cli/src/package/api.rs:1`
- [ ] Tests: the JSON of a fixture package is pinned byte for byte, and a package with an effectful top-level statement produces no effect (`rule:packaging/nothing-runs-before-the-program`). `crates/nvs-cli/tests/package_api.rs:1`

### 2. a removal, and an addition that breaks a caller, are breaks

**Does:** `diff` classifies every declaration and member change as breaking or additive.

**Split:** after the classifier lands, one writing subagent writes the fixture pairs of stages 2 and 3,
one directory per class in the standing decision's break list and each additive class.

- [ ] `nvs pkg diff <old> <new>` over two directories or two JSON files, one line per change, and `--json`. `crates/nvs-cli/src/package/diff.rs:1`, `crates/nvs-cli/src/package/mod.rs:1`
- [ ] A removed or renamed public declaration or member, and a lowered visibility, are breaks (`rule:packaging/only-a-major-may-break-the-api`). `crates/nvs-cli/src/package/diff.rs:1`
- [ ] A new method on a public interface, a new abstract method, a new enum case and a new required parameter are breaks. `crates/nvs-cli/src/package/diff.rs:1`
- [ ] A new class, a new concrete method and a new optional parameter are additive. `crates/nvs-cli/src/package/diff.rs:1`
- [ ] The fixture pairs, and a test that each gives its class. `crates/nvs-cli/tests/fixtures/api/removed-method/old/src/Api.nvs:1`, `crates/nvs-cli/tests/package_api.rs:1`

### 3. a parameter that narrows and a return that widens are breaks

**Does:** a type that changes direction is classified by which side of the call it is on.

- [ ] Parameter types may only widen and return types may only narrow; a union that gains a member, a nullable that appears and a class replaced by its parent are each classified by the side, over the two documents as the standing decision says. `crates/nvs-cli/src/package/diff.rs:1`, `crates/nvs-types/src/ty.rs:29`
- [ ] A change the comparison cannot decide is a break, and the line says so. `crates/nvs-cli/src/package/diff.rs:1`
- [ ] Tests: the type fixture pairs, one per direction and side, and one undecidable pair. `crates/nvs-cli/tests/package_api.rs:1`

### 4. the version step decides, and the commands have their feature proofs

**Does:** `diff` exits non-zero only on a break the version step forbids, and both commands have their
proofs.

**Split:** after the version step lands, one writing subagent writes the about page and the examples,
then a second the attack, each held to § *Text an end user reads* in `AGENTS.md`.

- [ ] `package/manifest.rs` reads `version` from each side's `package.toml`, and `--from` and `--to` replace it. `crates/nvs-cli/src/package/manifest.rs:1`
- [ ] `0.0.x` on either side is never checked, a major allows anything, and a minor or patch allows only additions; a test for each. `crates/nvs-cli/src/package/diff.rs:1`, `crates/nvs-cli/tests/package_api.rs:1`
- [ ] The heading `# nvs pkg api and nvs pkg diff` in `docs/reference/tools/35-packages.md`, which says the guarantee is about shape only. `docs/reference/tools/35-packages.md:1`
- [ ] The about page and the examples. `docs/examples/tools/packages/nvs-pkg-api-and-nvs-pkg-diff/about.md:1`
- [ ] The attack: a minor release that removes a method, with the version step trying to hide it, is refused. `tests/hostile/tools/packages/nvs-pkg-api-and-nvs-pkg-diff/01-a-break-in-a-minor.nvs:1`
- [ ] `bun nv proofs --id 'tools:packages/nvs-pkg-api-and-nvs-pkg-diff'` owes nothing. `docs/reference/tools/35-packages.md:1`
- [ ] `rule:packaging/only-a-major-may-break-the-api` is `shipped` with its cases in `guardedBy`, and `bun nv owners --closes api-compat` passes. `data/rules/packaging/only-a-major-may-break-the-api.json:3`

## Handoff

### State

The goal has not started. `nvs api diff` compares two OpenAPI documents and is the shape to follow
(`crates/nvs-cli/src/api_diff.rs:69`). There is no `nvs pkg` group, no `crates/nvs-cli/src/package/`,
and nothing extracts a package's API.

### Next

**Stage 1: the `nvs pkg` group and the extraction**, one file set: `crates/nvs-cli/src/main.rs`,
`crates/nvs-cli/src/package/`, `crates/nvs-cli/tests/package_api.rs`. No gate.

- [ ] `crates/nvs-cli/src/package/mod.rs` and `Command::Pkg` (`rule:packaging/package-toml-is-a-tool-input`). `crates/nvs-cli/src/main.rs:380`
- [ ] The extraction over resolved declarations (`rule:packaging/only-a-major-may-break-the-api`). `crates/nvs-hir/src/members.rs:136`
- [ ] The deterministic JSON with its schema version. `crates/nvs-cli/src/package/api.rs:1`
- [ ] Tests: the pinned JSON, and no effect from a top-level statement (`rule:packaging/nothing-runs-before-the-program`). `crates/nvs-cli/tests/package_api.rs:1`

### Backlog

- Stage 4 waits for `docs/reference/tools/35-packages.md` from goal `namespace-grants`; if it is absent, stage 4 creates the file with its own heading. `docs/goals/namespace-grants.md`
- The registry calls `nvs pkg diff` at publish in goal `registry-service` (`docs/goals/registry-service.md`).
- The website shows the API diff per version in goal `registry-website` (`docs/goals/registry-website.md`).
