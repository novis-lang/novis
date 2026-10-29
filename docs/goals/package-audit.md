---
looper: 1
title: "`nvs pkg audit` names every advisory a program's packages are under, and the bump that clears it"
summary: "The registry publishes a signed advisory feed whose every entry is in the log, nvs pkg audit reads it, names the minimum bump that clears each advisory and exits non-zero, compares the registry's checkpoint with a second published copy, warns on an autoload root no grant line mentions, has its feature proofs, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-cli/src/package/mod.rs, docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md]
stages:
  - id: 1
    title: the advisory feed's schema is stated, and the client verifies the feed against the log
    status: done
    context: [crates/nvs-cli/src/package/log.rs, crates/nvs-cli/src/package/note.rs, crates/nvs-cli/src/package/index.rs]
    extra:
      context:
        rules: [packaging/a-known-bad-version-is-retracted-not-deleted, packaging/the-operator-only-withholds, packaging/the-registry-keeps-an-append-only-log]
  - id: 5
    title: the registry publishes the signed advisory feed and names the second checkpoint copy
    status: done
    context: [apps/registry/src/App.nvs, apps/registry/src/Withhold.nvs, apps/registry/src/Releases.nvs, apps/registry/src/Index.nvs, apps/registry/src/Events.nvs]
    extra:
      context:
        rules: [packaging/a-known-bad-version-is-retracted-not-deleted, packaging/the-operator-only-withholds, packaging/the-registry-keeps-an-append-only-log]
  - id: 2
    title: "`nvs pkg audit` names the minimum bump that clears each advisory, and compares a second copy of the checkpoint"
    status: done
    context: [crates/nvs-cli/src/package/resolve.rs, crates/nvs-cli/src/package/lock.rs, crates/nvs-cli/src/package/index.rs, crates/nvs-cli/src/package/advisory.rs]
    extra:
      context:
        rules: [packaging/a-known-bad-version-is-retracted-not-deleted, packaging/resolution-takes-the-highest-minimum, packaging/the-registry-keeps-an-append-only-log, packaging/a-package-is-its-digest]
  - id: 3
    title: "`nvs pkg audit` warns on an autoload root no grant line mentions"
    status: done
    context: [crates/nvs-hir/src/autoload.rs, crates/nvs-config/src/grants.rs, crates/nvs-cli/src/main.rs, docs/rules/security/an-unmatched-namespace-holds-the-application.md]
    extra:
      context:
        rules: [security/an-unmatched-namespace-holds-the-application, security/grants-are-keyed-on-a-namespace]
  - id: 4
    title: "`nvs pkg audit` has its feature proofs, and its rules are shipped"
    status: open
    context: [docs/reference/tools/35-packages.md]
  - id: 6
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
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
the log's Ed25519 key as a signed note (`crates/nvs-cli/src/package/note.rs`, goal `registry-log`), and
every entry is also a log entry, so a feed that hides an advisory disagrees with the log. Stage 1 writes
the feed's schema into `rule:packaging/a-known-bad-version-is-retracted-not-deleted`'s fragment, and
the service writes to it in stage 5.

**`nvs pkg audit` reads the lock, never the network's idea of the graph.** For each locked version under an
advisory it prints the advisory, its reason and the smallest version that clears it, and exits
non-zero while any remain. A withheld version is reported the same way. An unlisted version is not an
advisory and is not reported.

**The second copy of the checkpoint** is published on another host, at a URL the registry's index root
names, so a private registry names its own. `nvs pkg audit` fetches both and fails when they disagree and
neither extends the other by a consistency proof. When no second copy is named, audit says so in one
line and does not fail.

**This goal adds the second-copy field to the index schema.** Goal `package-client` defines the index
and does not carry it. Stage 2 adds an optional `checkpoint_copy` URL to the index root, read by
`crates/nvs-cli/src/package/index.rs` and stated in `rule:packaging/a-package-is-its-digest`'s schema,
under the same schema version because absent means "no second copy". Stage 5 makes the service write
it from its own configuration. Why: the field has one reader, audit, and an optional field breaks no
client that predates it.

**The hand-copied-namespace warning** names each autoload root that lies under a directory no grant
line mentions. It is a warning, never an error, because the same shape describes a second source tree
of the application's own (`rule:security/an-unmatched-namespace-holds-the-application`).

**Lanes.** Stage 1 (`advisory.rs`) and stage 3 (`audit.rs`'s autoload walk) touch disjoint files and
can run in either order. Stage 2 also writes `audit.rs`, so stages 2 and 3 run one after the other,
whichever is unblocked first; stage 2 needs stage 1's reader. Stage 5 is the app lane
(`apps/registry/`), disjoint from the Rust stages; it shares `apps/registry/src/App.nvs`'s route wiring
with goal `registry-website`, so those merges are serial. The `nvs pkg audit` subcommand goes into
`crates/nvs-cli/src/main.rs`'s `Command` enum (`crates/nvs-cli/src/main.rs:373`), which goals
`api-compat`, `package-client`, `registry-log` and `registry-service` also edit. Stage 4 writes
`docs/reference/tools/35-packages.md`, where every package goal adds a heading.

**Gates.** Stage 3 needs goals `namespace-grants` and `package-client` reached:
`docs/goals/namespace-grants.md` and `docs/goals/package-client.md` are gone. Stage 1 needs goal
`registry-log` reached: `docs/goals/registry-log.md` is gone. Stage 2 needs stage 1 and
`crates/nvs-cli/src/package/lock.rs`. Stage 5 needs goal `registry-service` reached:
`docs/goals/registry-service.md` is gone. A session whose stage is gated takes the earliest open stage
whose gate holds, and ends `BLOCKED` naming the missing file when none does.

**The feed is derived from the log by `nvs registry log advisories`, and lives at `log/advisories`.**
It is JSON on one line signed as an index file is (`index::sign`), so `signed_at` and `checkpoint`
come from the same code, and each advisory is the log record's number and its entry. The service in
stage 5 runs the command after an append that logs an advisory and copies the file with the log's
others. Why: one writer, in Rust, whose every byte comes from records the log already signed, so the
service cannot write a feed the client's check rejects. Cost: the command reads every entry of the
log, once per advisory logged, on the operator's side.

**A retraction's `fixed_in` is in its log entry**, an optional field of `Entry::Retract` under schema
`v: 1`, left out of the canonical form when absent, so every existing log still verifies. Why: the
feed lists log entries only, so the fix audit names is one the log signed, never one the feed alone
claims. Stage 5 makes `Events::reasoned` write it.

**The feed lists `retract` and `withhold` entries only** (`advisory::is_advisory`). The log's
`advisory` kind is not in it: no route writes one, and its `versions` has no syntax. A goal that adds
that route adds the kind to the feed and a schema line to the rule.

**A feed may name a log shorter than the newest checkpoint.** `nvs registry log verify` accepts it
when its root is the log's at its size. Audit in stage 2 reads the records past the feed's size from
the log itself, so an append between two feed writes never fails a run, and nothing is hidden.

**The package page's *Advisories* section reads `release_states`, and links to the signed feed.**
The feed and the table hold the same retractions and withholds, each written in the transaction of
its event, so the page shows nothing the feed does not. Reading the feed instead would parse every
advisory of the registry on each page view, a request-path cost the latency priority does not
spend for no new fact. The link is `Site::feed()`: the first file host's `log/advisories`, or the
site's own `/log/advisories` with no hosts.

**The service writes the feed in `Index::run` after an append that logged a retraction or a
withhold, and in every daily run.** A feed write that fails returns `false` from the run without
undoing the append, as a failed send does, and the next daily run writes it. The feed is queued at
the index files' rank, after the log tiles it names.

**Audit checks a served log by computing its root from the records, not by checking tiles.**
`log::Fetched::walk` reads every entry bundle, hashes each record into an RFC 6962 stack, and
compares the root at the feed's size and at the checkpoint's size. Why: a record that is not the
signed tree's changes the root, so this is as strong as `log::verify`'s tile-by-tile check, reads
no hash tile, and needs no tile a host may have replaced. Cost: one fetch per 256 records and at
most 64 hashes held, once per audit run.

**A copy of the checkpoint that cannot be read fails the audit.** Only "no copy named" is the
one-line note. Why: the security priority; a split view that hides the copy would otherwise pass.
Audit also opens its registries without the seven-day index age limit, because it selects nothing.

**The warning walks every program under the application's folder, not one entry.** An
application's `package.toml` names no entry file, so audit runs `nvs_hir::Lender::walk` over each
`.nvs` file `lenders::sources` finds that starts a program, and looks each prefix of its own
`autoload` map up in `[grants]`. The application's own source tree matches no line either, so it
is named too, and a grant line for it silences it. Why: the stage asks for every root that holds
the application's authority only because no line matches, and a heuristic that guesses which tree
is the application's own would hide copied code placed beside it. Cost: one read of every `.nvs`
file under the folder and one name resolution per program, once per audit run.

**Trade-offs.** Nothing runs on the request path, and no program changes. Audit costs one feed fetch
and two checkpoint fetches per run. Users gain one command that CI runs, and one warning that tells
them which code holds the application's authority without a grant line.

## Stages

### 1. the advisory feed's schema is stated, and the client verifies the feed against the log

**Does:** the feed's schema is in its rule, and the client checks the feed's signature and its entries
against the log.

- [x] The feed's schema and signature, written into the rule's fragment (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`). `docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md:1`
  - [x] The schema, an example and the client's check in the fragment, and `docs/rules/packaging.md` rendered.
  - [x] `fixed_in` on the log's retract entry (`crates/nvs-cli/src/package/entry.rs`).
- [x] The client's reader and its check against the log: a bad signature, and an advisory the log holds but the feed lacks, are hard errors. `crates/nvs-cli/src/package/advisory.rs:1`, `crates/nvs-cli/src/package/log.rs:1`
  - [x] `Feed::open` and `Feed::against`, with unit tests.
  - [x] `nvs registry log advisories` writes the feed, and `log verify` checks it (`crates/nvs-cli/src/registry.rs`).
  - [x] The walk over a log directory is `log::verify`'s. The walk over a file host's tiles moved to stage 2's second-copy item.
- [x] Tests over a feed built with `nvs registry log append`: a valid feed, a changed byte, and a hidden advisory. `crates/nvs-cli/tests/package_audit.rs:1`

### 5. the registry publishes the signed advisory feed and names the second checkpoint copy

**Does:** the service writes the feed from its retractions and withholds, and its index root names the
second checkpoint copy when the operator configures one.

**Split:** after the session lands `Advisories.nvs`, a writing subagent writes its test.

- [x] The signed feed, written by the service from each retraction and withhold. `apps/registry/src/Advisories.nvs:1`, `apps/registry/src/Withhold.nvs:1`
  - [x] `Events::reasoned` writes a retraction's `fixed_in`, left out when there is none.
  - [x] `Log::advisories`, run by `NvsLog` as `nvs registry log advisories`.
  - [x] `Index::run` writes and queues the feed after an append that logged an advisory, and `Index::daily` always.
- [x] The index root's `checkpoint_copy`, from the service's configuration. `apps/registry/src/Index.nvs:1`
  - [x] `checkpointCopy` in `hosting.json`, an optional `https:` address, in `Site` and the app's README.
  - [x] `Index::root` writes `checkpoint_copy` only when it is set.
- [x] The package page's *Advisories* section shows each retraction and withhold from `release_states`, with a link to the signed feed (see *Standing decisions*). `apps/registry/src/PackagePage.nvs:301`
- [x] A test: a retraction and a withhold each appear in the feed, and the client's reader accepts it. `apps/registry/src/AdvisoriesTest.nvs:1`
  - [x] The entries the app logs are, in canonical form, the ones `crates/nvs-cli/tests/package_audit.rs` appends and checks with the client's reader.

### 2. `nvs pkg audit` names the minimum bump that clears each advisory, and compares a second copy of the checkpoint

**Does:** audit reports each advisory against the lock with its fix, and fails on a split view of the
log.

**Split:** after the session lands the report, a writing subagent writes the tests.

- [x] The report and its non-zero exit, with a `--json` form. `crates/nvs-cli/src/package/audit.rs:1`, `crates/nvs-cli/src/package/lock.rs:1`
  - [x] `Command::Audit` in `crates/nvs-cli/src/package/mod.rs`, with its help text.
  - [x] The bump: the smallest choosable version above the lock's, at least `fixed_in`, under no advisory (`minimum::choosable`).
- [x] The optional `checkpoint_copy` field in the index reader, and the schema line in `rule:packaging/a-package-is-its-digest`. `crates/nvs-cli/src/package/index.rs:1`, `docs/rules/packaging/a-package-is-its-digest.md:1`
- [x] The second-copy comparison, and the one line when none is named. `crates/nvs-cli/src/package/audit.rs:1`, `crates/nvs-cli/src/package/log.rs:1`
  - [x] `Seen::compare_copy`: same log, prefix or extension by a consistency proof, else the split-view error.
- [x] The feed checked against a file host's log, feeding `advisory::Feed::against`; the records past the feed's size read the same way. `crates/nvs-cli/src/package/log.rs:947`, `crates/nvs-cli/src/package/advisory.rs:1`
  - [x] `log::Fetched::walk` computes the root from the records (see *Standing decisions*), and `Seen::advisories` uses it.
- [x] Tests: a locked retracted version names its fixed-in version and exits non-zero, a withheld one the same, an unlisted one is silent, and two disagreeing copies fail. `crates/nvs-cli/tests/package_audit.rs:1`

### 3. `nvs pkg audit` warns on an autoload root no grant line mentions

**Does:** audit names each autoload root whose namespace holds the application's authority only
because no grant line matches it.

- [x] The `nvs pkg audit` subcommand, if stage 2 has not added it. `crates/nvs-cli/src/main.rs:373`
- [x] The walk over the program's autoload roots against the grant table (`rule:security/an-unmatched-namespace-holds-the-application`). `crates/nvs-cli/src/package/audit.rs:1`, `crates/nvs-hir/src/autoload.rs:199`, `crates/nvs-config/src/grants.rs:1`
  - [x] `AutoloadMap::prefixes` and `Lender::map` in `nvs-hir`.
  - [x] `audit::unmatched` walks every program under the folder with `Lender::walk`, and `warn` prints each prefix on standard error; `--json` has `unmatched`.
- [x] Tests: a root under a granted prefix is silent, an ungranted one warns, and the exit stays zero. `crates/nvs-cli/tests/package_audit.rs:1`

### 4. `nvs pkg audit` has its feature proofs, and its rules are shipped

**Does:** audit is a feature with its proofs, and every rule this goal owns is `shipped`.

**Split:** after the session writes the heading, one writing subagent writes `about.md` and the
examples `bun nv proofs --id 'tools:packages/nvs-pkg-audit'` lists, then a second writes the attack.

- [ ] The heading `# nvs pkg audit` in `docs/reference/tools/35-packages.md`. `docs/reference/tools/35-packages.md:1`
- [ ] `about.md` and the examples the proofs command lists. `docs/examples/tools/packages/nvs-pkg-audit/about.md:1`
- [ ] The attack: a registry hides an advisory from its feed that its log holds. `tests/hostile/tools/packages/nvs-pkg-audit/01-a-hidden-advisory.nvs:1`
- [ ] `rule:packaging/a-known-bad-version-is-retracted-not-deleted` and `rule:security/an-unmatched-namespace-holds-the-application` are `shipped`, and `bun nv owners --closes package-audit` passes. `data/rules/packaging/a-known-bad-version-is-retracted-not-deleted.json:1`, `data/rules/security/an-unmatched-namespace-holds-the-application.json:1`

### 6. the goal is verified once, and every failure is fixed

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

Stages 1, 5, 2 and 3 are done. `nvs pkg audit [--json]` (`crates/nvs-cli/src/package/audit.rs`,
wired in `package/mod.rs`'s `Command`) reads the lock, checks each registry's feed against the
served log, compares the root file's `checkpoint_copy`, and reports each advisory with its bump.
It then warns on standard error about each autoload prefix no `[grants]` line matches
(`audit::unmatched`, over `Lender::walk` and the new `AutoloadMap::prefixes`), and `--json` lists
them under `unmatched`. Tests run once by name and green:
`audit_warns_on_an_autoload_prefix_no_grant_line_matches` and the edited
`audit_json_names_the_advisory_and_the_matching_copy`. No test is expected to go red.

The sweep's `env-secret-proofs` and nightly-run 40662443492 findings are stale, as last session
found: the log is from Oct 22 and exited 0, no check of that name exists, and the run was a manual
dispatch cancelled whole. Nothing to fix.

### Next

Stage 4, on `docs/reference/tools/35-packages.md` and the proofs.
- [ ] The heading `# nvs pkg audit`, covering the advisory report, the second copy and the warning. `docs/reference/tools/35-packages.md:1`
- [ ] `about.md` and the examples `bun nv proofs --id 'tools:packages/nvs-pkg-audit'` lists. `docs/examples/tools/packages/nvs-pkg-audit/about.md:1`
- [ ] The attack: a registry hides an advisory from its feed. `tests/hostile/tools/packages/nvs-pkg-audit/01-a-hidden-advisory.nvs:1`
- [ ] Both rules `shipped`, and the *Not on disk* paragraph of `docs/rules/security/an-unmatched-namespace-holds-the-application.md` rewritten: fetch writes the lines and audit warns now. `data/rules/security/an-unmatched-namespace-holds-the-application.json:1`

### Backlog

- From goal `registry-website`: a search row restored from a backup is checked only indirectly, since
  `BackupTest::aRestoreBringsBackEveryRelease` restores releases, not `search`. Stage 5 touches the
  registry app; if it changes `apps/registry/src/Backup.nvs`, add a test that a restored registry
  still finds a package.
