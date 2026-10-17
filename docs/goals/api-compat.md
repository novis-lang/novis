---
looper: 1
title: "`nvs` extracts a package's public API and finds every break between two versions"
summary: "nvs pkg api writes a package's public API from the checked program without running any of it, nvs pkg diff classifies every change between two versions and exits non-zero on a break that the version step does not allow, the registry and the website have one JSON form to call, the commands have their feature proofs, rule packaging/only-a-major-may-break-the-api is shipped, and the checks below pass."
context: [crates/nvs-cli/src/api_diff.rs, crates/nvs-cli/src/main.rs, docs/rules/packaging/only-a-major-may-break-the-api.md]
stages:
  - id: 1
    title: "`nvs pkg api` writes a package's public API without running it"
    status: open
    context: [crates/nvs-hir/src/members.rs, crates/nvs-hir/src/hierarchy.rs, crates/nvs-hir/src/symbol.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api, packaging/nothing-runs-before-the-program]
  - id: 2
    title: a removal, and an addition that breaks a caller, are breaks
    status: open
    context: [crates/nvs-cli/src/api_diff.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api]
  - id: 3
    title: a parameter that narrows and a return that widens are breaks
    status: open
    context: [crates/nvs-cli/src/api_diff.rs, crates/nvs-types/src/ty.rs]
    extra:
      context:
        rules: [packaging/only-a-major-may-break-the-api]
  - id: 4
    title: the version step decides, and the commands have their feature proofs
    status: open
    context: [docs/reference/tools/35-packages.md, docs/rules/packaging/below-1-0-the-breaking-slot-moves-left.md]
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
extra:
  milestone: M15
---

# `nvs` extracts a package's public API and finds every break between two versions

## Why

A package's version is a promise: from `0.1.0` on, a minor or patch release does not break the public
API of the release before it. A promise nobody checks is a guess, so `nvs` checks it statically at
publish, and the registry refuses a release that breaks it
(`rule:packaging/only-a-major-may-break-the-api`). This goal builds the extraction and the comparison
as `nvs` subcommands. The registry service (goal `registry-service`) and its website (goal
`registry-website`) call the same commands, so the publisher, the registry and the reader see one
answer. It comes before the client because nothing else in the packaging work depends on the client,
and every publish depends on this.

## Standing decisions

**Versions are `MAJOR.MINOR.PATCH`, read literally.** `0.0.x` is experimental and never checked. From
the first `0.1.0` or higher, the API is stable: a minor or patch release may add and widen, and may not
break. A major may change anything. This is not the toolchain's own scheme
(`rule:packaging/below-1-0-the-breaking-slot-moves-left`), and the new rule says so in one sentence.

**The break list is the brief's, and it errs towards breaking**, as `nvs pkg api diff` does
(`crates/nvs-cli/src/api_diff.rs:24`). A break is: removing or renaming anything public, a new method
on a public interface, a new abstract method, a new enum case (it breaks an exhaustive `match`), a
narrowed parameter, a widened return and a new required parameter. Anything the comparison cannot
decide is a break. There is no suppression mechanism.

**The guarantee is about API shape only.** Behaviour may change in any release, and the reference
chapter says so.

**The API is extracted from the checked program, never from text and never by running it.** The
extraction reads declarations after name resolution, so an alias, an import or a formatting change is
not a change. A type is written in its resolved, normalised form, and two versions compare a type by
its fully qualified name.

**The commands are `nvs pkg api <dir>` and `nvs pkg diff <old> <new>`.** `api` prints the API
as JSON with a schema version. `diff` takes two directories or two such JSON files, prints one line per
change with its class, has a `--json` form, and exits non-zero on a break the version step does not
allow. The version step is read from each side's `package.toml` `version`, or given with
`--from` and `--to`.

**Trade-offs.** No program changes, and nothing runs on the request path. A publish pays one compile
of each version. A package author loses the freedom to break a stable API in a minor, which is the
point. Simplicity: one check the user already expects from semantic versioning, applied by a tool
rather than by convention.

## Stages

### 1. `nvs pkg api` writes a package's public API without running it

**Does:** the public classes, interfaces, enums, functions and constants of a package, with every
public and protected member's signature, are written as deterministic JSON.

- [ ] The extraction over resolved declarations: visibility, `abstract`, `final`, `readonly`, enum cases, parameters with their types, defaults present or not, and return types. `crates/nvs-hir/src/members.rs:1`, `crates/nvs-hir/src/hierarchy.rs:1`
- [ ] The subcommand, beside `nvs pkg api`, and a test that a package with an effectful top-level statement produces no effect (`rule:packaging/nothing-runs-before-the-program`). `crates/nvs-cli/src/main.rs:714`
- [ ] The output is byte-identical across runs and platforms, and a test pins it. `crates/nvs-cli/tests/package_api.rs:1`

### 2. a removal, and an addition that breaks a caller, are breaks

**Does:** `diff` classifies every declaration and member change as breaking or additive.

- [ ] A removed or renamed public declaration or member, and a lowered visibility, are breaks (`rule:packaging/only-a-major-may-break-the-api`). `crates/nvs-cli/src/package/diff.rs:1`
- [ ] A new method on a public interface, a new abstract method, a new enum case and a new required parameter are breaks. `crates/nvs-cli/src/package/diff.rs:1`
- [ ] A new class, a new concrete method and a new optional parameter are additive. `crates/nvs-cli/tests/package_api.rs:1`

### 3. a parameter that narrows and a return that widens are breaks

**Does:** a type that changes direction is classified by which side of the call it is on.

- [ ] Parameter types may only widen and return types may only narrow; a union that gains a member, a nullable that appears and a class replaced by its parent are each classified by the side. `crates/nvs-cli/src/package/diff.rs:1`, `crates/nvs-types/src/ty.rs:1`
- [ ] A change the comparison cannot decide is a break, and the line says so. `crates/nvs-cli/tests/package_api.rs:1`

### 4. the version step decides, and the commands have their feature proofs

**Does:** `diff` exits non-zero only on a break the version step forbids, and both commands have their
proofs.

- [ ] `0.0.x` on either side is never checked, a major allows anything, and a minor or patch allows only additions. `crates/nvs-cli/src/package/diff.rs:1`
- [ ] The heading `# nvs pkg api and nvs pkg diff` in `docs/reference/tools/35-packages.md`, and `bun nv proofs --id 'tools:packages/nvs-pkg-api-and-nvs-pkg-diff'` owes nothing. `docs/reference/tools/35-packages.md:1`
- [ ] The rule is `shipped` with its cases in `guardedBy`, and `bun nv owners --closes api-compat` passes. `data/rules/packaging/only-a-major-may-break-the-api.json:1`

## Handoff

### State

The goal is new. No stage is started. `nvs pkg api diff` compares two OpenAPI documents and is the shape
to follow; nothing extracts a package's API yet.

### Next

Stage 1, on the resolved declarations and the CLI:
- the extraction over resolved declarations (`rule:packaging/only-a-major-may-break-the-api`). `crates/nvs-hir/src/members.rs:1`
- the `nvs pkg api` subcommand. `crates/nvs-cli/src/main.rs:714`

### Backlog

- The registry calls `nvs pkg diff` at publish in goal `registry-service` (`docs/goals/registry-service.md`).
- The website shows the API diff per version in goal `registry-website` (`docs/goals/registry-website.md`).
