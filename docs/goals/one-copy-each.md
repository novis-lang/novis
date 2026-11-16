---
looper: 1
title: code that was written several times is written once
summary: "Reached when each listed block of repeated code has one home, with no change in behaviour and fewer lines than before."
context: [crates/nvs-ir/src/lower/control.rs, crates/nvs-stdlib/src/db/registry.rs, crates/nvs-lsp/src/hints.rs, crates/nvs-lsp/src/semantic.rs, docs/agent/coding.md]
stages:
  - id: 1
    title: "the loop forms share one preamble"
    status: done
    context: [crates/nvs-ir/src/lower/control.rs, crates/nvs-ir/src/lower/mod.rs]
  - id: 2
    title: "a connection's and a transaction's members are declared once"
    status: done
    context: [crates/nvs-stdlib/src/db/registry.rs, crates/nvs-stdlib/src/classes.rs]
  - id: 3
    title: "the tree walks and the case parser have one home each"
    status: done
    context: [crates/nvs-lsp/src/hints.rs, crates/nvs-lsp/src/semantic.rs, crates/nvs-lsp/src/case.rs, crates/nvs-test/src/case.rs, crates/nvs-types/src/deprecated.rs, crates/nvs-types/src/retrieval.rs, crates/nvs-types/src/test_code.rs]
  - id: 4
    title: "the small helpers are written once"
    status: done
    context: [crates/nvs-runtime/src/callable.rs, crates/nvs-stdlib/src/encoding.rs, crates/nvs-stdlib/src/random.rs, crates/nvs-codegen/src/emit.rs]
  - id: 5
    title: "the server tests of nvs-cli share their support"
    status: done
    context: [crates/nvs-cli/tests/live_config.rs, crates/nvs-test/src/lib.rs]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks: []
---

# code that was written several times is written once

## Why

A scan of every Rust source file outside the tests looked for runs of 14 or more identical lines,
with comments and blank lines taken out. It found 91 runs in 703 files, 1,640 lines in all. That is
little for a tree this size, and it sits in a few places:

- **445 lines in `crates/nvs-ir/src/lower/control.rs`.** The preamble of a loop, which collects the
  reassigned locals and opens the header block, is written at `:133`, `:309`, `:515`, `:1088` and
  `:1482`. The back-edge block at `:1254` is repeated at `:1564`.
- **219 lines in `crates/nvs-stdlib/src/db/registry.rs`.** `Connection` at `:333` and `Transaction`
  at `:715` declare the same members with the same parameter tables.
- **69 lines between `crates/nvs-lsp/src/hints.rs:124` and `crates/nvs-lsp/src/semantic.rs:572`.**
  Both walk every statement and expression kind.
- **55 lines between `crates/nvs-lsp/src/case.rs:315` and `crates/nvs-test/src/case.rs:509`.** Both
  parse the sections of a case file.
- **Three copies of one namespace walk** in `crates/nvs-types/src/deprecated.rs:438`,
  `retrieval.rs:238` and `test_code.rs:80`.
- **Three copies of a 20-line block** in `crates/nvs-runtime/src/callable.rs:771`, `:832` and
  `:1006`.

Readers found smaller repeats the scan cannot see: a hex encoder in five files
(`crates/nvs-stdlib/src/encoding.rs:1617`, `random.rs:769`, `server.rs:153`, `queue.rs:252`,
`json.rs:1391`), the same "expected a string" accessor in seven stdlib files
(`crates/nvs-stdlib/src/csrf.rs:306` and six more), and three copies of the load-flags-and-branch
probe in `crates/nvs-codegen/src/emit.rs:1556`, `:1765` and `:3973`.

The `nvs-cli` server tests carry their own support: a `Server` struct in 16 test files, a `TempDir`
in 8, and the `BOOT` and `POLL` constants in 15.

A repeated block is a place where a fix lands in one copy and not the other. That is the reason to
remove it. Speed is not.

## Standing decisions

**Limits: what stops this goal.**

- **Fewer lines, or it does not land.** A stage's commit removes more lines than it adds, counted
  by `git diff --stat` over `crates/`. An item whose shared form is longer than the copies it
  replaces is declined, and the copies stay.
- **No behaviour changes.** No test's expected output, no snapshot and no diagnostic text changes
  in this goal. An item that needs one changed is declined, because the copies were not the same
  code after all.
- **No new abstraction with one user.** A shared function has at least two callers on the day it
  lands. No trait is added for a block of under 30 lines.
- **The list is closed.** Only the blocks named in the stages are joined. The scan is not run again
  to find more.
- **An item gets one attempt, and a stage two sessions,** counted in `### State`. At the ceiling the
  open items move to `### Backlog` as declined.
- **Stage 5 is bounded by files, not by completeness.** It moves the `Server` boot and `TempDir`
  into one support module and converts test files in the order listed until its two sessions end.
  Files it did not reach keep their own copy, and `### Backlog` lists them.

**The database row loop is not in this goal.** Goal `fewer-round-trips` writes it once when it
shares the row keys.

**The ancestor walk and `qname_segments` are not in this goal.** Goal `lean-members-and-checker`
joins them when it stops their clones.

**Generated files stay generated.** `crates/nvs-stdlib/src/classes.rs` is written by `bun nv
render`, so stage 2 changes the source table and renders.

**The `Connection` and `Transaction` members are one `const` row each, listed by both classes.**
Rust cannot splice one `const` slice into another, so each shared row is its own `const`, as
`TRANSACTION_ROW` already was, and both instance lists name the seven. `bun nv gaps` reads the
class literals as text, so it follows a row named on its own line to every class that lists it.

**An unreachable internal fatal is not a diagnostic, so its text may be joined.** The seven
string accessors each wrote their own `Fault::fatal` text for a tag the checker already refuses
with `E0401`. `crate::string_arg` writes one form, `<who> expected a `string`, got tag N`, and
each caller still names its member and parameter. No test reads any of the seven texts.

**The test support lives in `crates/nvs-test/`,** as a module the test files import. It is a helper
crate, so no test binaries are merged.

**Lanes:** every stage touches different files. Each can run as a side goal, after the goal named
in its decision above has landed.

## Stages

### 1. the loop forms share one preamble

**Does:** write the loop preamble and the back-edge block of the IR lowering once.

- [x] `crates/nvs-ir/src/lower/control.rs:133`: one function for the preamble, called at `:309`,
  `:515`, `:1088` and `:1482` (`rule:errors/propagation`).
- [x] `crates/nvs-ir/src/lower/control.rs:1254`: one function for the back-edge block, called at
  `:1564` (`rule:errors/propagation`).
- [x] `crates/nvs-ir/src/lower/mod.rs:1730`: the block at `:2143` calls the one at `:1730`
  (`rule:errors/propagation`).

### 2. a connection's and a transaction's members are declared once

**Does:** declare the shared database members in one list.

- [x] `crates/nvs-stdlib/src/db/registry.rs:333`: one member list, used by `Connection` and by
  `Transaction` at `:715`, and `bun nv render` leaves `crates/nvs-stdlib/src/classes.rs`
  unchanged (`rule:core-api/reference-card`).
  - [x] the six statement rows are one `const` each, listed by both classes
  - [x] `bun nv gaps` attributes a row listed by name to every class that lists it
  - [x] build and clippy green, and `bun nv render` leaves `classes.rs` unchanged

### 3. the tree walks and the case parser have one home each

**Does:** share the statement walk of the two editor passes, the case-file parser, and the
namespace walk of three checker passes.

- [x] `crates/nvs-lsp/src/hints.rs:124`: one walker with a visitor, used by
  `crates/nvs-lsp/src/semantic.rs:572` (`rule:programs/memory-priority`).
  - [x] `crates/nvs-lsp/src/walk.rs` holds the shared walk as trait `Walk`
  - [x] hints and semantic tokens override the nodes they read and call the shared children walk
  - [x] build and clippy green, with fewer lines
- [x] `crates/nvs-test/src/case.rs:509`: the section parser is public, and
  `crates/nvs-lsp/src/case.rs:315` calls it (`rule:testing/feature-proofs`).
  - [x] `nvs_test::section::aux_path` holds the path check, with each format's reserved names
  - [x] both case parsers call it, with every message unchanged
- [x] `crates/nvs-types/src/deprecated.rs:438`: one namespace walk, used by
  `crates/nvs-types/src/retrieval.rs:238` and `crates/nvs-types/src/test_code.rs:80`
  (`rule:programs/memory-priority`).
  - [x] `deprecated::each_declaration` walks the namespaces and imports
  - [x] the three table builders read their declarations through it

### 4. the small helpers are written once

**Does:** give the hex encoder, the string-argument accessor, the callable block and the probe
branch one home each.

- [x] `crates/nvs-stdlib/src/encoding.rs:1617`: one hex encoder, used by `random.rs:769`,
  `server.rs:153`, `queue.rs:252` and `json.rs:1391` in `crates/nvs-stdlib/src/`
  (`rule:programs/memory-priority`).
  - [x] `encoding::hex_pair` and `encoding::HEX_DIGITS` hold the digits
  - [x] the four copies call it, and build and clippy are green
- [x] `crates/nvs-stdlib/src/csrf.rs:306`: one string-argument accessor, used by
  `signed_cookie.rs:292`, `session.rs:1061`, `ratelimit.rs:713`, `log.rs:297`, `html.rs:574` and
  `metrics.rs:413` in `crates/nvs-stdlib/src/` (`rule:errors/helper-abi`).
  - [x] `crate::string_arg` in `lib.rs` takes the member's name as a lazy `Display`
  - [x] the seven private accessors are gone, and build and clippy are green
- [x] `crates/nvs-runtime/src/callable.rs:771`: one function for the block at `:832` and `:1006`
  (`rule:errors/helper-abi`).
  - [x] `callable_class` returns the object and its descriptor
  - [x] its three callers use it, and build and clippy are green
- [x] `crates/nvs-codegen/src/emit.rs:1556`: one function for the probe branch at `:1765` and
  `:3973` (`rule:errors/helper-abi`).
  - [x] `emit_probe_branch` emits the load and the branch, creating blocks in the old order
  - [x] the statement, edge and call probes use it, and build and clippy are green

### 5. the server tests of nvs-cli share their support

**Does:** move the `Server` boot, its `Drop`, the address parse and `TempDir` into one support
module, and convert the test files that use them.

**Split:** an `Explore` subagent lists every test file under `crates/` that declares
`struct Server`, `struct TempDir`, `const BOOT` or `const POLL`, as `file.rs:NN`, before the first
edit. Writing subagents then convert disjoint files side by side, and the session builds once.

- [x] `crates/nvs-test/src/lib.rs:1`: the support module, written from
  `crates/nvs-cli/tests/live_config.rs:60` (`rule:testing/feature-proofs`).
- [x] `crates/nvs-cli/tests/live_config.rs:60`: this file and the others on the list use the
  module, largest first (`rule:testing/feature-proofs`).

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 5 are done and clippy-clean, with no test run. Lines over `crates/`: stage 1 +156 -313,
stage 2 +179 -254, stage 3 +698 -936, stage 4 +108 -251, stage 5 +552 -931. One session each.

- Stages 1 to 4: `crates/nvs-ir/src/lower/`'s loop helpers, `db/registry.rs`'s rows, trait `Walk`,
  `aux_path`, `each_declaration`, `hex_pair`, `string_arg`, `callable_class`, `emit_probe_branch`.
- Stage 5: `nvs_test::serve` (`BOOT`, `POLL`, `Served`, `succeeds`) and `nvs_test::temp::TempDir`,
  which boxes the caller's `nvs_repo::scratch` guard. `nvs-types` takes `nvs-test` as a dev-dependency.

The sweep's `env-secret-proofs` finding names a retired check with no log; it needed no change.

Tests that could go red in stage 6: the `insta` snapshots under `crates/nvs-ir/src/lower/snapshots/`;
`db::transaction`'s roster test; `nvs-lsp`'s `tests/hints.rs`, `semantic.rs` and the `.lspt` corpus;
both `case.rs` test sets; `random.rs`, `server.rs`'s trace id, `tests/queue_sqlite.rs` and the JSON
`\u` escapes; `html.rs` and `callable.rs`. For stage 5, every `nvs-cli` server test its commit
names, `nvs-lsp`'s six `TempDir` tests and `nvs-types`' `tests/common`. `reload_follows_links.rs`
gives `Served` its data folder to remove, since the program folder outlives the server.

### Next

Stage 6, the gate: start `looper verify` with `run_in_background`, fix every failure in one pass,
run it again, then the goal-end gates of AGENTS.md § *In the loop*.

- [ ] `crates/nvs-test/src/serve.rs:1`: `looper verify` over the whole goal, every failure fixed
  at its cause, then green on the second run.
- [ ] `docs/goals/one-copy-each.md:1`: `bun nv verify --doc`, `bun nv owners --closes
  one-copy-each` and `bun nv ci --check`, each red fixed.

### Backlog

- Declined before the run: joining the per-driver code of `crates/nvs-db/` (reset, begin, commit,
  the read loop). Each driver speaks another protocol, and a shared form would hide where they
  differ.
- Declined before the run: joining the repeated lists and set-up of `tools/nv/`. No block there
  reaches 14 identical lines, and goal `cheaper-pipeline` edits the same files.
- Left by stage 4: `crates/nvs-stdlib/src/topic.rs:477` writes hex digits with its own table. It
  was not on the stage's list, and the list is closed.
- Left by stage 5, because `Served` cannot express their boot: `serve_isolation.rs` (stdout joins
  the stderr buffer, and asserts read `child.try_wait`), `telemetry_e2e.rs` (stderr read line by
  line over a channel), `schedule_events.rs` (sends a stop signal by `child.id`),
  `serve_ownership.rs` (a boot that must fail) and `telemetry.rs`. Each now takes `BOOT` from
  `nvs_test::serve`. `registry_build.rs` (120s) and `builtin_warm.rs` (180s) keep their own
  `BOOT`. The `#[cfg(test)]` `TempDir`s in `crates/nvs-hir/src/requires.rs` and
  `crates/nvs-lsp/src/document.rs` were not on the list.
