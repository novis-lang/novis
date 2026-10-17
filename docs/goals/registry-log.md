---
looper: 1
title: The registry keeps an append-only log, and the client refuses a fork
summary: "A Rust log library writes and verifies C2SP tlog-tiles with signed-note Ed25519 checkpoints, inclusion and consistency proofs, and one entry kind per event the registry records, nvs registry log append and verify are the commands the service calls, the client checks every fetched digest's inclusion, keeps the newest checkpoint it saw and refuses an inconsistent one as a fork, the log has its feature proofs, rule packaging/the-registry-keeps-an-append-only-log is shipped, and the checks below pass."
context: [crates/nvs-cli/src/package/mod.rs, docs/rules/packaging/the-registry-keeps-an-append-only-log.md]
stages:
  - id: 1
    title: tiles, signed checkpoints, and inclusion and consistency proofs
    status: open
    context: [Cargo.toml, docs/rules/packaging/somebody-elses-specification-is-a-crate.md, docs/rules/packaging/a-c-dependency-answers-two-questions.md]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, packaging/somebody-elses-specification-is-a-crate, packaging/a-c-dependency-answers-two-questions]
  - id: 2
    title: one entry kind per recorded event, and the commands the service calls
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-config/src/data.rs]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, packaging/publishing-never-overwrites]
  - id: 3
    title: the client checks inclusion, keeps the newest checkpoint and refuses a fork
    status: open
    context: [crates/nvs-cli/src/package/fetch.rs, crates/nvs-cli/src/cache.rs, crates/nvs-config/src/data.rs]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, config/ownership-is-the-trust-boundary, packaging/the-lockfile-holds-every-digest]
  - id: 4
    title: the log has its feature proofs, and its rule is shipped
    status: open
    context: [docs/reference/tools/35-packages.md]
checks:
  - id: log-proofs
    stage: 4
    name: the transparency log owes no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:packages/the-transparency-log"]
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, registry-log]
    reach: true
    floor: false
extra:
  milestone: M15
---

# The registry keeps an append-only log, and the client refuses a fork

## Why

A registry that can show one user a different archive than another, or quietly change who owns a
package, is a single point of compromise. A transparency log makes every such change public and
permanent: each publish, owner change, token, unlist, retraction, withhold and confirmation is an
entry, and a client that saw one version of the log refuses any later version that does not extend it.
This goal builds the log in Rust, once, so the client and the registry service (goal
`registry-service`) run the same code. It follows goal `package-client` because the client is where
inclusion is checked.

## Standing decisions

**The formats are C2SP's.** The log is tlog-tiles, a checkpoint is a signed note, and the signature is
Ed25519. Outside witnesses can co-sign a checkpoint later with no format change, and none are required
now.

**An existing crate implements the formats if one meets the dependency rules.** It must pass
`rule:packaging/a-c-dependency-answers-two-questions` and `cargo deny`. If none does, the module is
written in this tree, and `rule:packaging/the-registry-keeps-an-append-only-log` argues the exception
against `rule:packaging/somebody-elses-specification-is-a-crate`. Ed25519 signing and verifying go
through `ring`, which `Core\Jwt` and `Core\Crypto` already use (`Cargo.toml:798`).

**The entry kinds are a closed list.** A publish (name, version, digest, repository and commit), an
ownership change, a token created or revoked, a repository binding changed, an unlist and its reversal,
a retraction, a withhold, an advisory, a release confirmed early, and a stable API confirmed. Each
entry is canonical JSON with a schema version, and the log is the only record of order.

**The byte work is `nvs registry log`.** `append` adds entries to a log directory and signs a new
checkpoint with a key file; `verify` checks a log directory from scratch. The key file is read under
the ownership check the artifact cache uses (`rule:config/ownership-is-the-trust-boundary`). These
commands are operator surface for the registry service and print no help in the user's command list.

**The client stores one checkpoint per registry** in the data folder, under the same ownership check.
A new checkpoint must come with a consistency proof from the stored one. An inconsistent one is a hard
error that names the registry and says the log was forked, and nothing is fetched.

**Trade-offs.** Nothing runs on the request path, and no program changes. A fetch pays one inclusion
proof per archive and one consistency proof per checkpoint, each a few hashes and one tile read.
Memory: a stored checkpoint is a few hundred bytes per registry. Security gains the most: a registry
cannot rewrite history without every client that saw it refusing.

## Stages

### 1. tiles, signed checkpoints, and inclusion and consistency proofs

**Does:** a log directory can be appended to, its checkpoint signed and verified, and both proofs
produced and checked.

- [ ] The dependency choice made and recorded in the rule, with `cargo deny` green. `Cargo.toml:1`
- [ ] Tiles, the signed-note checkpoint and both proofs, with tests over the C2SP test vectors. `crates/nvs-cli/src/package/log.rs:1`

### 2. one entry kind per recorded event, and the commands the service calls

**Does:** the closed list of entry kinds has its canonical form, and `nvs registry log append` and
`verify` work on a directory.

- [ ] The entry kinds and their canonical JSON (`rule:packaging/the-registry-keeps-an-append-only-log`). `crates/nvs-cli/src/package/log.rs:1`
- [ ] `nvs registry log append` and `nvs registry log verify`, with the key file under the ownership check. `crates/nvs-cli/src/main.rs:361`, `crates/nvs-config/src/data.rs:446`

### 3. the client checks inclusion, keeps the newest checkpoint and refuses a fork

**Does:** every archive the client fetches is proved to be in the log, and a log that does not extend
the one the client saw is refused.

- [ ] An inclusion proof for every fetched digest, checked before the archive is written under `vendor/`. `crates/nvs-cli/src/package/fetch.rs:1`
- [ ] The stored checkpoint per registry, its consistency proof, and the fork error. `crates/nvs-cli/src/package/log.rs:1`, `crates/nvs-cli/src/cache.rs:1793`
- [ ] The fixture registry from goal `package-client` gains a log, and a test forks it. `crates/nvs-cli/tests/package_client.rs:1`

### 4. the log has its feature proofs, and its rule is shipped

**Does:** the log is a feature with its proofs, and its rule is `shipped`.

- [ ] The heading `# The transparency log` in `docs/reference/tools/35-packages.md`, owing nothing. `docs/reference/tools/35-packages.md:1`
- [ ] The attack: a registry serves a rewritten log to a client that saw the old one. `tests/hostile/tools/packages/the-transparency-log/01-a-forked-log.nvs:1`
- [ ] The rule is `shipped`, and `bun nv owners --closes registry-log` passes. `data/rules/packaging/the-registry-keeps-an-append-only-log.json:1`

## Handoff

### State

The goal is new. No stage is started. It needs the client's fetch path from goal `package-client`.

### Next

Stage 1, the log library:
- choose the crate or argue the exception (`rule:packaging/the-registry-keeps-an-append-only-log`). `Cargo.toml:1`
- tiles, checkpoints and proofs. `crates/nvs-cli/src/package/log.rs:1`

### Backlog

- The second published copy of the checkpoint is compared by `nvs pkg audit` in goal `package-audit` (`docs/goals/package-audit.md`).
