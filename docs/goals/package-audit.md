---
looper: 1
title: "`nvs pkg audit` names every advisory a program's packages are under, and the bump that clears it"
summary: "The registry publishes a signed advisory feed whose every entry is in the log, nvs pkg audit reads it, names the minimum bump that clears each advisory and exits non-zero, compares the registry's checkpoint with a second published copy, warns on an autoload root no grant line mentions, has its feature proofs, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-cli/src/package/mod.rs, docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md]
stages:
  - id: 1
    title: the registry publishes a signed advisory feed, and the client verifies it
    status: open
    context: [crates/nvs-cli/src/package/log.rs, crates/nvs-cli/src/package/index.rs, apps/registry/src/Withhold.nvs, apps/registry/src/Releases.nvs]
    extra:
      context:
        rules: [packaging/a-known-bad-version-is-retracted-not-deleted, packaging/the-operator-only-withholds, packaging/the-registry-keeps-an-append-only-log]
  - id: 2
    title: "`nvs pkg audit` names the minimum bump that clears each advisory, and compares a second copy of the checkpoint"
    status: open
    context: [crates/nvs-cli/src/package/resolve.rs, crates/nvs-cli/src/package/lock.rs]
    extra:
      context:
        rules: [packaging/a-known-bad-version-is-retracted-not-deleted, packaging/resolution-takes-the-highest-minimum, packaging/the-registry-keeps-an-append-only-log]
  - id: 3
    title: "`nvs pkg audit` warns on an autoload root no grant line mentions"
    status: open
    context: [crates/nvs-hir/src/autoload.rs, crates/nvs-config/src/capability.rs, docs/rules/security/an-unmatched-namespace-holds-the-application.md]
    extra:
      context:
        rules: [security/an-unmatched-namespace-holds-the-application, security/grants-are-keyed-on-a-namespace]
  - id: 4
    title: "`nvs pkg audit` has its feature proofs, and its rules are shipped"
    status: open
    context: [docs/reference/tools/35-packages.md]
checks:
  - id: audit-proofs
    stage: 4
    name: nvs pkg audit owes no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:packages/nvs-pkg-audit"]
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, package-audit]
    reach: true
    floor: false
---

# `nvs pkg audit` names every advisory a program's packages are under, and the bump that clears it

## Why

Minimal version selection never upgrades on its own, so a fix nobody asked for does not arrive by
itself (`rule:packaging/resolution-takes-the-highest-minimum`). Retraction plus audit is the answer: a
known-bad version is retracted with a reason and a fixed-in version, and `nvs pkg audit` in CI names what
to bump and fails until it is bumped. This goal also closes the one gap goal `namespace-grants` leaves
open by design: hand-copied code whose namespace no grant line mentions holds the application's
authority, and audit is where that is reported.

## Standing decisions

**The advisory feed is a signed file the registry publishes**, with one entry per retraction and per
withhold, each carrying its reason and, for a retraction, its fixed-in version. The feed is signed with
the log's Ed25519 key as a signed note, and every entry is also a log entry (goal `registry-log`), so a
feed that hides an advisory disagrees with the log.

**`nvs pkg audit` reads the lock, never the network's idea of the graph.** For each locked version under an
advisory it prints the advisory, its reason and the smallest version that clears it, and exits
non-zero while any remain. A withheld version is reported the same way. An unlisted version is not an
advisory and is not reported.

**The second copy of the checkpoint** is published on another host, at a URL the registry's index root
names, so a private registry names its own. `nvs pkg audit` fetches both and fails when they disagree and
neither extends the other by a consistency proof. When no second copy is named, audit says so in one
line and does not fail.

**The hand-copied-namespace warning** names each autoload root that lies under a directory no grant
line mentions. It is a warning, never an error, because the same shape describes a second source tree
of the application's own (`rule:security/an-unmatched-namespace-holds-the-application`).

**Trade-offs.** Nothing runs on the request path, and no program changes. Audit costs one feed fetch
and two checkpoint fetches per run. Users gain one command that CI runs, and one warning that tells
them which code holds the application's authority without a grant line.

## Stages

### 1. the registry publishes a signed advisory feed, and the client verifies it

**Does:** the service writes the feed from its retractions and withholds, and the client checks its
signature and its entries against the log.

- [ ] The feed's schema and signature, written by the service. `apps/registry/src/Advisories.nvs:1`, `apps/registry/src/Withhold.nvs:1`
- [ ] The client's reader and its check against the log (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`). `crates/nvs-cli/src/package/advisory.rs:1`, `crates/nvs-cli/src/package/log.rs:1`

### 2. `nvs pkg audit` names the minimum bump that clears each advisory, and compares a second copy of the checkpoint

**Does:** audit reports each advisory against the lock with its fix, and fails on a split view of the
log.

- [ ] The report and its non-zero exit, with a `--json` form. `crates/nvs-cli/src/package/audit.rs:1`, `crates/nvs-cli/src/package/lock.rs:1`
- [ ] The second-copy comparison. `crates/nvs-cli/src/package/audit.rs:1`, `crates/nvs-cli/src/package/log.rs:1`

### 3. `nvs pkg audit` warns on an autoload root no grant line mentions

**Does:** audit names each autoload root whose namespace holds the application's authority only
because no grant line matches it.

- [ ] The walk over the program's autoload roots against the grant table (`rule:security/an-unmatched-namespace-holds-the-application`). `crates/nvs-cli/src/package/audit.rs:1`, `crates/nvs-hir/src/autoload.rs:139`

### 4. `nvs pkg audit` has its feature proofs, and its rules are shipped

**Does:** audit is a feature with its proofs, and every rule this goal owns is `shipped`.

- [ ] The heading `# nvs pkg audit` in `docs/reference/tools/35-packages.md`, owing nothing. `docs/reference/tools/35-packages.md:1`
- [ ] The attack: a registry hides an advisory from its feed that its log holds. `tests/hostile/tools/packages/nvs-audit/01-a-hidden-advisory.nvs:1`
- [ ] Both rules `shipped`, and `bun nv owners --closes package-audit` passes. `data/rules/packaging/a-known-bad-version-is-retracted-not-deleted.json:1`

## Handoff

### State

The goal is new. No stage is started. It needs goals `package-client`, `registry-log` and
`registry-service` reached.

### Next

Stage 1, the feed:
- the service writes the signed feed (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`). `apps/registry/src/Advisories.nvs:1`
- the client reads and checks it. `crates/nvs-cli/src/package/advisory.rs:1`

### Backlog

- None.
