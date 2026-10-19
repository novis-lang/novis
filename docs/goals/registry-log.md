---
looper: 1
title: The registry keeps an append-only log, and the client refuses a fork
summary: "A Rust log library writes and verifies C2SP tlog-tiles with signed-note Ed25519 checkpoints, inclusion and consistency proofs, and one entry kind per event the registry records, nvs registry log append and verify are the commands the service calls, the client checks every fetched digest's inclusion, keeps the newest checkpoint it saw and refuses an inconsistent one as a fork, the log has its feature proofs, rule packaging/the-registry-keeps-an-append-only-log is shipped, and the checks below pass."
context: [crates/nvs-cli/src/package/mod.rs, docs/rules/packaging/the-registry-keeps-an-append-only-log.md]
stages:
  - id: 1
    title: tiles, and inclusion and consistency proofs
    status: open
    context: [Cargo.toml, deny.toml, crates/nvs-cli/Cargo.toml, docs/rules/packaging/somebody-elses-specification-is-a-crate.md, docs/rules/packaging/a-c-dependency-answers-two-questions.md]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, packaging/somebody-elses-specification-is-a-crate, packaging/a-c-dependency-answers-two-questions]
  - id: 5
    title: a checkpoint is a signed note under an Ed25519 key
    status: open
    context: [Cargo.toml, crates/nvs-cli/Cargo.toml, crates/nvs-cli/src/package/log.rs]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, packaging/somebody-elses-specification-is-a-crate]
  - id: 2
    title: one entry kind per recorded event, and the commands the service calls
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-config/src/data.rs, crates/nvs-cli/src/package/log.rs, crates/nvs-cli/src/package/note.rs]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, packaging/publishing-never-overwrites, config/ownership-is-the-trust-boundary]
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
`rule:packaging/a-c-dependency-answers-two-questions` and `cargo deny`. The candidates are the
`tlog_tiles` and `signed_note` crates and any other an `Explore` subagent finds on crates.io. If none
passes, the module is written in this tree, and `rule:packaging/the-registry-keeps-an-append-only-log`
argues the exception against `rule:packaging/somebody-elses-specification-is-a-crate`. Ed25519 signing
and verifying go through `ring`, which `Core\Jwt` and `Core\Crypto` already use (`Cargo.toml:798`), and
`nvs-cli` gains `ring.workspace = true`.

**Two modules.** Tiles and proofs are `crates/nvs-cli/src/package/log.rs`; the signed note is
`crates/nvs-cli/src/package/note.rs`, so goal `package-audit` signs its advisory feed with the same
code. Simplicity: one file per C2SP specification.

**The test vectors are vendored.** They are copied from the C2SP specifications and from the Go
reference implementation's tests (`golang.org/x/mod/sumdb/tlog` and `sumdb/note`) into
`crates/nvs-cli/tests/fixtures/tlog/`, and each file's header names its source URL and commit. No test
fetches anything. Why: a test that needs the network is a test CI cannot trust.

**The entry kinds are a closed list.** A publish (name, version, digest, repository and commit), an
ownership change, a token created or revoked, a repository binding changed, an unlist and its reversal,
a retraction, a withhold, an advisory, a release confirmed early, and a stable API confirmed. Each
entry is canonical JSON with a schema version, and the log is the only record of order.

**The byte work is `nvs registry log`.** `append` adds entries to a log directory and signs a new
checkpoint with a key file; `verify` checks a log directory from scratch. The key file is read under
the ownership check of the data folder (`crates/nvs-config/src/data.rs:446`,
`rule:config/ownership-is-the-trust-boundary`). These commands are operator surface for the registry
service, hidden from the user's command list.

**The client stores one checkpoint per registry** under `<data>/registry/`, a new accessor beside
`logs()` (`crates/nvs-config/src/data.rs:163`), read under the same ownership check the artifact cache
applies (`crates/nvs-cli/src/cache.rs:1258`). A new checkpoint must come with a consistency proof from
the stored one. An inconsistent one is a hard error that names the registry and says the log was
forked, and nothing is fetched.

**Lanes.** Stage 1 (`log.rs`, `Cargo.toml`) and stage 5 (`note.rs`) touch disjoint files and can run in
either order or as side goals; both add to `crates/nvs-cli/Cargo.toml`, so the second to land rebases
it. Stage 2 needs both: it writes `log.rs` and `crates/nvs-cli/src/main.rs`'s `Command` enum
(`crates/nvs-cli/src/main.rs:373`), which goals `api-compat` and `package-client` also edit for the
`pkg` group, so its merge is serial with theirs. Stage 3 writes `package-client`'s `fetch.rs` and
`data.rs`, and runs after stages 1, 5 and 2. Stage 4 writes `docs/reference/tools/35-packages.md`,
where every package goal adds a heading, so it merges serially.

**Gates.** Stages 1, 5 and 2 need `crates/nvs-cli/src/package/mod.rs` to exist (goal `api-compat`
creates it with the `nvs pkg` group). Stage 3 needs `crates/nvs-cli/src/package/fetch.rs` and
`crates/nvs-cli/tests/fixtures/registry/` (goal `package-client` stages 3 and 4). When a stage's gate
does not hold, the session takes the earliest stage whose gate holds; when none holds, it ends
`BLOCKED` naming the missing file.

**Trade-offs.** Nothing runs on the request path, and no program changes. A fetch pays one inclusion
proof per archive and one consistency proof per checkpoint, each a few hashes and one tile read.
Memory: a stored checkpoint is a few hundred bytes per registry. Security gains the most: a registry
cannot rewrite history without every client that saw it refusing.

## Stages

### 1. tiles, and inclusion and consistency proofs

**Does:** a log directory can be appended to as tiles, and both proofs are produced and checked.

**Split:** before the session starts, an `Explore` subagent checks each candidate crate against
`deny.toml`, the two questions of `rule:packaging/a-c-dependency-answers-two-questions` and the C2SP
tlog-tiles text, and returns one line per crate with its verdict. A second one lists the C2SP and Go
test vectors with their URLs and commits.

- [ ] The dependency choice, recorded in `rule:packaging/the-registry-keeps-an-append-only-log` with its reason, and `cargo deny check` green. `Cargo.toml:798`, `docs/rules/packaging/the-registry-keeps-an-append-only-log.md:1`
- [ ] The vendored tlog vectors, each file naming its source and commit. `crates/nvs-cli/tests/fixtures/tlog/tiles.json:1`
- [ ] Leaf and node hashing, the tile layout and its paths, and append (`rule:packaging/the-registry-keeps-an-append-only-log`). `crates/nvs-cli/src/package/log.rs:1`
- [ ] Inclusion and consistency proofs, produced and checked. `crates/nvs-cli/src/package/log.rs:1`
- [ ] Tests: each vector's hashes, proofs and tile paths, and a proof with one hash changed refused. `crates/nvs-cli/src/package/log.rs:1`

### 5. a checkpoint is a signed note under an Ed25519 key

**Does:** a checkpoint (origin, size, root hash) is signed as a C2SP signed note and verified, and a
key file is read and written in the note key format.

- [ ] `ring` added to `nvs-cli`. `crates/nvs-cli/Cargo.toml:1`
- [ ] The vendored signed-note vectors, each naming its source and commit. `crates/nvs-cli/tests/fixtures/tlog/note.json:1`
- [ ] The checkpoint body, the signed note, its key id and the verify step (`rule:packaging/the-registry-keeps-an-append-only-log`). `crates/nvs-cli/src/package/note.rs:1`
- [ ] Tests: each vector verifies, and a note with a changed byte, an unknown key id or a second signature line from an unknown key is handled as the C2SP text says. `crates/nvs-cli/src/package/note.rs:1`

### 2. one entry kind per recorded event, and the commands the service calls

**Does:** the closed list of entry kinds has its canonical form, and `nvs registry log append` and
`verify` work on a directory.

- [ ] The entry kinds and their canonical JSON with a schema version (`rule:packaging/the-registry-keeps-an-append-only-log`). `crates/nvs-cli/src/package/log.rs:1`
- [ ] `nvs registry log append` and `nvs registry log verify`, hidden from the user's command list. `crates/nvs-cli/src/main.rs:373`
- [ ] The key file read under the ownership check (`rule:config/ownership-is-the-trust-boundary`). `crates/nvs-config/src/data.rs:446`
- [ ] Tests that drive the binary: append then verify, a tampered tile refused by verify, an entry outside the closed list refused, and a key file writable by another account refused. `crates/nvs-cli/tests/registry_log.rs:1`

### 3. the client checks inclusion, keeps the newest checkpoint and refuses a fork

**Does:** every archive the client fetches is proved to be in the log, and a log that does not extend
the one the client saw is refused.

- [ ] An inclusion proof for every fetched digest, checked before the archive is written under `vendor/`. `crates/nvs-cli/src/package/fetch.rs:1`
- [ ] The checkpoint directory accessor beside `logs()`, read under the ownership check. `crates/nvs-config/src/data.rs:163`
- [ ] The stored checkpoint per registry, its consistency proof, and the fork error naming the registry. `crates/nvs-cli/src/package/log.rs:1`
- [ ] The fixture registry from goal `package-client` gains a log built with `nvs registry log append`, and tests cover an archive absent from the log and a forked log. `crates/nvs-cli/tests/package_client.rs:1`

### 4. the log has its feature proofs, and its rule is shipped

**Does:** the log is a feature with its proofs, and its rule is `shipped`.

**Split:** after the session writes the heading, one writing subagent writes `about.md` and the
examples `bun nv proofs --id 'tools:packages/the-transparency-log'` lists, then a second writes the
attack. Both follow AGENTS.md § *Text an end user reads*.

- [ ] The heading `# The transparency log` in `docs/reference/tools/35-packages.md`. `docs/reference/tools/35-packages.md:1`
- [ ] `about.md` and the examples the proofs command lists. `docs/examples/tools/packages/the-transparency-log/about.md:1`
- [ ] The attack: a registry serves a rewritten log to a client that saw the old one. `tests/hostile/tools/packages/the-transparency-log/01-a-forked-log.nvs:1`
- [ ] The rule is `shipped` with the stage 1, 5, 2 and 3 tests in its `guardedBy`, and `bun nv owners --closes registry-log` passes. `data/rules/packaging/the-registry-keeps-an-append-only-log.json:1`

## Handoff

### State

The goal is new. No stage is started. `crates/nvs-cli/src/package/mod.rs` does not exist yet, so every
stage's gate is open; goal `api-compat` creates it.

### Next

Stage 1, the tiles and proofs, on `Cargo.toml`, `crates/nvs-cli/src/package/log.rs` and
`crates/nvs-cli/tests/fixtures/tlog/`. Gate: `crates/nvs-cli/src/package/mod.rs` exists.
- [ ] The dependency choice recorded, `cargo deny check` green (`rule:packaging/somebody-elses-specification-is-a-crate`). `Cargo.toml:798`
- [ ] The vendored tlog vectors with their sources. `crates/nvs-cli/tests/fixtures/tlog/tiles.json:1`
- [ ] Hashing, tiles and append (`rule:packaging/the-registry-keeps-an-append-only-log`). `crates/nvs-cli/src/package/log.rs:1`
- [ ] Inclusion and consistency proofs. `crates/nvs-cli/src/package/log.rs:1`
- [ ] Their tests over the vectors. `crates/nvs-cli/src/package/log.rs:1`

### Backlog

- Stage 5, the signed note, is a lane beside stage 1 (`crates/nvs-cli/src/package/note.rs`).
- The second published copy of the checkpoint is compared by `nvs pkg audit` in goal `package-audit` (`docs/goals/package-audit.md`).
