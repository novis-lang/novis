---
looper: 1
title: The registry keeps an append-only log, and the client refuses a fork
summary: "A Rust log library writes and verifies C2SP tlog-tiles with signed-note Ed25519 checkpoints, inclusion and consistency proofs, and one entry kind per event the registry records, nvs registry log append and verify are the commands the service calls, the client checks every fetched digest's inclusion, keeps the newest checkpoint it saw and refuses an inconsistent one as a fork, every index file is a signed note with a time and a checkpoint that nvs registry index sign writes and the client checks against a pinned verifier key and a seven-day age bound for selecting, the log has its feature proofs, rule packaging/the-registry-keeps-an-append-only-log is shipped, and the checks below pass."
context: [crates/nvs-cli/src/package/mod.rs, docs/rules/packaging/the-registry-keeps-an-append-only-log.md]
stages:
  - id: 1
    title: tiles, and inclusion and consistency proofs
    status: done
    context: [Cargo.toml, deny.toml, crates/nvs-cli/Cargo.toml, docs/rules/packaging/somebody-elses-specification-is-a-crate.md, docs/rules/packaging/a-c-dependency-answers-two-questions.md]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, packaging/somebody-elses-specification-is-a-crate, packaging/a-c-dependency-answers-two-questions]
  - id: 5
    title: a checkpoint is a signed note under an Ed25519 key
    status: done
    context: [Cargo.toml, crates/nvs-cli/Cargo.toml, crates/nvs-cli/src/package/log.rs]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, packaging/somebody-elses-specification-is-a-crate]
  - id: 2
    title: one entry kind per recorded event, and the commands the service calls
    status: done
    context: [crates/nvs-cli/src/main.rs, crates/nvs-config/src/data.rs, crates/nvs-cli/src/package/log.rs, crates/nvs-cli/src/package/note.rs]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, packaging/publishing-never-overwrites, config/ownership-is-the-trust-boundary]
  - id: 3
    title: the client checks inclusion, keeps the newest checkpoint and refuses a fork
    status: done
    context: [crates/nvs-cli/src/package/fetch.rs, crates/nvs-cli/src/cache.rs, crates/nvs-config/src/data.rs]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, config/ownership-is-the-trust-boundary, packaging/the-lockfile-holds-every-digest]
  - id: 6
    title: every index file is signed with a time, and the client checks it against a pinned key and an age bound
    status: open
    context: [crates/nvs-cli/src/package/index.rs, crates/nvs-cli/src/package/note.rs, crates/nvs-cli/src/package/manifest.rs, docs/rules/packaging/a-vendor-resolves-from-one-registry.md]
    extra:
      context:
        rules: [packaging/the-registry-keeps-an-append-only-log, packaging/a-vendor-resolves-from-one-registry, packaging/registry-hosting-is-configuration]
  - id: 4
    title: the log has its feature proofs, and its rule is shipped
    status: open
    context: [docs/reference/tools/35-packages.md]
  - id: 7
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
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
`nvs-cli` gains `ring.workspace = true`. Stage 1 settled it: neither crate passes. Both are at 0.2
with one maintainer and on `sha2` 0.10. `tlog_tiles` pulls in `url` and ICU, and `signed_note` a
second `ed25519-dalek`. So `log.rs` and `note.rs` are both written here, and the rule argues it.

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

**Every index file is a signed note with a time** (`rule:packaging/the-registry-keeps-an-append-only-log`).
`nvs registry index sign <dir>` signs the root file and each package's index file in a directory with
the key file, the current time and the size and root hash of the directory's newest checkpoint; the
note's text is the file's JSON, which carries both. The service runs it after each publish and once a
day over every file. The client verifies each index file with the registry's pinned verifier key: the
default registry's is a constant in `crates/nvs-cli/src/package/index.rs`, and a vendor that
`package.toml` maps to another registry names its key beside its URL. A file that fails the signature,
names a checkpoint not consistent with the stored one, or is signed more than an hour ahead of the
client's clock is refused, and the next file host is tried. `nvs pkg add`, `update` and `outdated`
refuse a file signed more than seven days ago. A build from the lock and `nvs pkg fetch --locked` read
no state they select from and are unaffected. Why: the index decides which version is released, so
whoever serves it must not be able to change it, and an old copy may hide a fix for seven days at most.

**The entries are their own module, and arrive on standard input.** The closed list is
`crates/nvs-cli/src/package/entry.rs`, beside `log.rs`, because `log.rs` is C2SP's format and the
entry kinds are the registry's own. `nvs registry log append` reads one JSON entry per line on
standard input and adds nothing when one line fails. Each field is a non-empty string, a `version`
is a package version and a `digest` is the package's BLAKE3 hex digest, and the log carries no time:
its order is the only order it claims. The ownership change and the token change are one kind each,
with `change` naming the direction. Unlist and its reversal are `unlist` and `relist`. Appends to
one directory are serialized by the service. Why: simplicity, one file per format, and a batch that
lands whole or not at all.

**Lanes.** Stage 1 (`log.rs`, `Cargo.toml`) and stage 5 (`note.rs`) touch disjoint files and can run in
either order or as side goals; both add to `crates/nvs-cli/Cargo.toml`, so the second to land rebases
it. Stage 2 needs both: it writes `log.rs` and `crates/nvs-cli/src/main.rs`'s `Command` enum
(`crates/nvs-cli/src/main.rs:373`), which goals `api-compat` and `package-client` also edit for the
`pkg` group, so its merge is serial with theirs. Stage 3 writes `package-client`'s `fetch.rs` and
`data.rs`, and runs after stages 1, 5 and 2. Stage 6 writes `index.rs`, `manifest.rs` and the
`nvs registry index sign` arm of the `Command` enum, and runs after stage 3. Stage 4 writes `docs/reference/tools/35-packages.md`,
where every package goal adds a heading, so it merges serially.

**Gates.** Stages 1, 5 and 2 need `crates/nvs-cli/src/package/mod.rs` to exist (goal `api-compat`
creates it with the `nvs pkg` group). Stage 3 needs `crates/nvs-cli/src/package/fetch.rs` and
`crates/nvs-cli/tests/fixtures/registry/` (goal `package-client` stages 3 and 4). Stage 6 needs stage
3's stored checkpoint (`crates/nvs-cli/src/package/log.rs` checks consistency). When a stage's gate
does not hold, the session takes the earliest stage whose gate holds; when none holds, it ends
`BLOCKED` naming the missing file.

**Trade-offs.** Nothing runs on the request path, and no program changes. A fetch pays one inclusion
proof per archive and one consistency proof per checkpoint, each a few hashes and one tile read.
Memory: a stored checkpoint is a few hundred bytes per registry. Security gains the most: a registry
cannot rewrite history without every client that saw it refusing, and whoever serves the index cannot
change a version's state or hide a fix for more than seven days. The cost is a signature check per
index file read, and the service writing every index file again once a day. Usability: selecting a
new version needs an index signed in the last seven days, while every build from the lock works
through any outage.

**The client's log checks are `crates/nvs-cli/src/package/proof.rs`, run in
`Registries::archive`.** Every path that reads a registry archive (fetch, vendor, add, update and
their reports) goes through that one function, so the check cannot be skipped by a new caller.
The log is under each file host's `log/`, as `nvs registry log append` writes a directory. Each
index line names its publish record as `"log": <n>`, required, and the record must publish the
same slug, version, digest, repository and commit. A checkpoint's origin is its verifier key's
name, so the pinned key is the only thing a `[registries]` line adds. The kept checkpoint is
`<data>/registry/<hash>.checkpoint`, beside the kept root file: `Folder::registry` is the
checkpoint directory, and no second accessor was added. A host whose checkpoint does not open, is
shorter than the kept one, or whose tiles do not lead to its own root is skipped; a signed
checkpoint the kept one is not a prefix of is the fork error, and no other host is tried. Why:
security first, one place to check, and no new folder to secure.

**A registry with no verifier key gives no archive.** The `key` of a `[registries]` line is
required once an archive is read, and the default registry's key is
`manifest::DEFAULT_KEY`, `None` until goal `registry-service` makes the production key. No key
was generated here, because its private half belongs to the service's operator. Index-only
commands such as `nvs pkg outdated` still run without a key until stage 6 makes every index read
need one. `nvs registry key <file> --name <n>` makes a key file and prints the verifier key.

## Stages

### 1. tiles, and inclusion and consistency proofs

**Does:** a log directory can be appended to as tiles, and both proofs are produced and checked.

**Split:** before the session starts, an `Explore` subagent checks each candidate crate against
`deny.toml`, the two questions of `rule:packaging/a-c-dependency-answers-two-questions` and the C2SP
tlog-tiles text, and returns one line per crate with its verdict. A second one lists the C2SP and Go
test vectors with their URLs and commits.

- [x] The dependency choice, recorded in `rule:packaging/the-registry-keeps-an-append-only-log` with its reason, and `cargo deny check` green. `Cargo.toml:798`, `docs/rules/packaging/the-registry-keeps-an-append-only-log.md:1`
  - [x] Both candidate crates checked against `deny.toml` and the two rules.
  - [x] The verdict and its reason in the rule, rendered, and `cargo deny check` green.
- [x] The vendored tlog vectors, each file naming its source and commit. `crates/nvs-cli/tests/fixtures/tlog/tiles.json:1`
  - [x] `rfc6962.json` from transparency-dev's `merkle`, copied from the pinned source.
  - [x] `tiles.json` from C2SP and Go's `tilePaths`, with C2SP's grammar's invalid paths.
- [x] Leaf and node hashing, the tile layout and its paths, and append (`rule:packaging/the-registry-keeps-an-append-only-log`). `crates/nvs-cli/src/package/log.rs:1`
  - [x] Hashing and the stored hash index.
  - [x] `Tile::path` and `Tile::parse`, and the tile a hash is read from.
  - [x] `append`, which never changes a written file.
- [x] Inclusion and consistency proofs, produced and checked. `crates/nvs-cli/src/package/log.rs:1`
- [x] Tests: each vector's hashes, proofs and tile paths, and a proof with one hash changed refused. `crates/nvs-cli/src/package/log.rs:1`

### 5. a checkpoint is a signed note under an Ed25519 key

**Does:** a checkpoint (origin, size, root hash) is signed as a C2SP signed note and verified, and a
key file is read and written in the note key format.

- [x] `ring` added to `nvs-cli`. `crates/nvs-cli/Cargo.toml:1`
- [x] The vendored signed-note vectors, each naming its source and commit. `crates/nvs-cli/tests/fixtures/tlog/note.json:1`
- [x] The checkpoint body, the signed note, its key id and the verify step (`rule:packaging/the-registry-keeps-an-append-only-log`). `crates/nvs-cli/src/package/note.rs:1`
- [x] Tests: each vector verifies, and a note with a changed byte, an unknown key id or a second signature line from an unknown key is handled as the C2SP text says. `crates/nvs-cli/src/package/note.rs:1`

### 2. one entry kind per recorded event, and the commands the service calls

**Does:** the closed list of entry kinds has its canonical form, and `nvs registry log append` and
`verify` work on a directory.

- [x] The entry kinds and their canonical JSON with a schema version (`rule:packaging/the-registry-keeps-an-append-only-log`). `crates/nvs-cli/src/package/entry.rs:1`
- [x] `nvs registry log append` and `nvs registry log verify`, hidden from the user's command list. `crates/nvs-cli/src/registry.rs:1`
- [x] The key file read under the ownership check (`rule:config/ownership-is-the-trust-boundary`). `crates/nvs-cli/src/registry.rs:1`
- [x] Tests that drive the binary: append then verify, a tampered tile refused by verify, an entry outside the closed list refused, and a key file writable by another account refused. `crates/nvs-cli/tests/registry_log.rs:1`

### 3. the client checks inclusion, keeps the newest checkpoint and refuses a fork

**Does:** every archive the client fetches is proved to be in the log, and a log that does not extend
the one the client saw is refused.

- [x] An inclusion proof for every fetched digest, checked before the archive is written under `vendor/`. `crates/nvs-cli/src/package/fetch.rs:1`
  - [x] `log::Fetched`, a `HashReader` over tiles a file host serves, with the full-tile fallback.
  - [x] Each index line's `log` record number, and `Seen::include` in `Registries::archive`.
- [x] The checkpoint directory accessor beside `logs()`, read under the ownership check. `crates/nvs-config/src/data.rs:163`
  - [x] `Folder::registry` is that directory; its doc says so, and `proof::read_kept` runs the check.
- [x] The stored checkpoint per registry, its consistency proof, and the fork error naming the registry. `crates/nvs-cli/src/package/log.rs:1`
  - [x] `Seen::open`: kept checkpoint, host skip, consistency proof, fork error.
  - [x] `kept_name` names a `file:` registry by its canonical path.
- [x] The fixture registry from goal `package-client` gains a log built with `nvs registry log append`, and tests cover an archive absent from the log and a forked log. `crates/nvs-cli/tests/package_client.rs:1`
  - [x] `fixture.rs` and the binary tests' registries append each publish to a log.
  - [x] `an_archive_absent_from_the_log_is_refused`, `a_forked_log_is_refused`, `a_log_that_grew_is_accepted`.

### 6. every index file is signed with a time, and the client checks it against a pinned key and an age bound

**Does:** `nvs registry index sign` signs the root file and every index file, and the client refuses an
index file that is unsigned, signed by another key, bound to an inconsistent checkpoint or too old to
select from.

- [ ] `nvs registry index sign <dir>` with the key file, the time and the newest checkpoint, hidden from the user's command list (`rule:packaging/the-registry-keeps-an-append-only-log`). `crates/nvs-cli/src/package/note.rs:1`, `crates/nvs-cli/src/main.rs:373`
- [ ] The pinned verifier key: the default registry's constant, and a key beside each other registry's URL in `package.toml` (`rule:packaging/a-vendor-resolves-from-one-registry`). `crates/nvs-cli/src/package/index.rs:1`, `crates/nvs-cli/src/package/manifest.rs:1`
  - [x] `Manifest::registry_key` and `manifest::DEFAULT_KEY` (stage 3 needed them for the log).
  - [ ] Every index read needs the key, not only an archive read.
- [ ] The reader checks each file's signature, its checkpoint against the stored one and its time, and tries the next file host on a refusal; an inconsistent checkpoint stays the fork error. `crates/nvs-cli/src/package/index.rs:1`
- [ ] `nvs pkg add`, `update` and `outdated` refuse a file signed more than seven days ago, naming the registry and the time; a build from the lock and `nvs pkg fetch --locked` do not. `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] The fixture registry is signed with `nvs registry index sign`, and tests cover an unsigned file, another key, a changed state with the old signature, a file eight days old refused for `add` and accepted for `--locked`, a file two hours ahead, and the second file host used after a bad signature. `crates/nvs-cli/tests/package_client.rs:1`

### 4. the log has its feature proofs, and its rule is shipped

**Does:** the log is a feature with its proofs, and its rule is `shipped`.

**Split:** after the session writes the heading, one writing subagent writes `about.md` and the
examples `bun nv proofs --id 'tools:packages/the-transparency-log'` lists, then a second writes the
attack. Both follow AGENTS.md § *Text an end user reads*.

- [ ] The heading `# The transparency log` in `docs/reference/tools/35-packages.md`. `docs/reference/tools/35-packages.md:1`
- [ ] `about.md` and the examples the proofs command lists. `docs/examples/tools/packages/the-transparency-log/about.md:1`
- [ ] The attacks: a registry serves a rewritten log to a client that saw the old one, and a file host marks a pending release as released. `tests/hostile/tools/packages/the-transparency-log/01-a-forked-log.nvs:1`, `tests/hostile/tools/packages/the-transparency-log/02-a-changed-index.nvs:1`
- [ ] The rule is `shipped` with the stage 1, 5, 2, 3 and 6 tests in its `guardedBy`, and `bun nv owners --closes registry-log` passes. `data/rules/packaging/the-registry-keeps-an-append-only-log.json:1`

### 7. the goal is verified once, and every failure is fixed

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

Stages 1, 5, 2 and 3 are done. Stage 3: `crates/nvs-cli/src/package/proof.rs` is the client's
side of the log. `Seen::open` reads the kept checkpoint (`<data>/registry/<hash>.checkpoint`, under
the ownership check), takes the first file host whose `log/checkpoint` opens with the pinned key
and extends it, and gives the fork error otherwise. `Seen::include` proves an index line's `log`
record publishes that slug, version, digest, repository and commit. Both run in
`Registries::archive` (`crates/nvs-cli/src/package/resolve.rs`). `log::Fetched` reads tiles from
a host. Each index line needs `"log"`, and a `[registries]` line needs `key` once an archive is
read. `manifest::DEFAULT_KEY` is `None`. The hidden `nvs registry key <file> --name <n>` makes a
key. `kept_name` names a `file:` registry by its canonical path, which the fork test found.

Run once by name, all green: `package::` unit tests (68), `--test package_fetch` (9),
`--test package_download` (9), `--test package_client` (16, three new). `log.rs` and `note.rs`
still keep `expect(dead_code)` and the build shows them fulfilled: `Tile::parse` and
`Verifier::id` are still uncalled outside tests. Stage 6 or 4 removes the lines once nothing is dead.

The sweep's reds were stale again: the `env-secret-proofs` log is Oct 22's with exit 0, and the Miri
reds are run 40474302703, which `c33a8b8f7` and `e461612ce` fixed.

Expected red: none known. A test elsewhere that builds an index file by hand without `"log"`, or
fetches from a registry with no `key`, would go red. The grep found only the three files fixed here.

### Next

Stage 6, the signed index files, on `crates/nvs-cli/src/package/index.rs`,
`crates/nvs-cli/src/registry.rs`, `crates/nvs-cli/src/package/fixture.rs` and
`crates/nvs-cli/tests/package_client.rs`. Gate: stage 3's kept checkpoint exists (`proof.rs`).
- [ ] `nvs registry index sign <dir> --key <file>`: each index file and the root file as a signed note whose text is the JSON with `signed_at` and the checkpoint's `size` and `root`. `crates/nvs-cli/src/registry.rs:1`
- [ ] `Registry::open` and `Registry::index` open each file with the pinned key (`Manifest::registry_key`), check its checkpoint against `Seen`'s, refuse a time more than an hour ahead, and try the next host. Move the key requirement from `Registries::archive` to `Registries::registry`. `crates/nvs-cli/src/package/index.rs:385`
- [ ] The seven-day bound for `add`, `update` and `outdated`, and none for `fetch --locked`. `crates/nvs-cli/src/package/resolve.rs:1`
- [ ] The fixture and the binary tests' registries sign their files, and the tests stage 6 lists. `crates/nvs-cli/src/package/fixture.rs:1`

### Backlog

- The default registry's verifier key (`manifest::DEFAULT_KEY`) is set by goal `registry-service` when it makes the production key with `nvs registry key`.
- The second published copy of the checkpoint is compared by `nvs pkg audit` in goal `package-audit` (`docs/goals/package-audit.md`).
