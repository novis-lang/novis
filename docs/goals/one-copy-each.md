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
    status: open
    context: [crates/nvs-lsp/src/hints.rs, crates/nvs-lsp/src/semantic.rs, crates/nvs-lsp/src/case.rs, crates/nvs-test/src/case.rs, crates/nvs-types/src/deprecated.rs, crates/nvs-types/src/retrieval.rs, crates/nvs-types/src/test_code.rs]
  - id: 4
    title: "the small helpers are written once"
    status: open
    context: [crates/nvs-runtime/src/callable.rs, crates/nvs-stdlib/src/encoding.rs, crates/nvs-stdlib/src/random.rs, crates/nvs-codegen/src/emit.rs]
  - id: 5
    title: "the server tests of nvs-cli share their support"
    status: open
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

- [ ] `crates/nvs-lsp/src/hints.rs:124`: one walker with a visitor, used by
  `crates/nvs-lsp/src/semantic.rs:572` (`rule:programs/memory-priority`).
- [ ] `crates/nvs-test/src/case.rs:509`: the section parser is public, and
  `crates/nvs-lsp/src/case.rs:315` calls it (`rule:testing/feature-proofs`).
- [ ] `crates/nvs-types/src/deprecated.rs:438`: one namespace walk, used by
  `crates/nvs-types/src/retrieval.rs:238` and `crates/nvs-types/src/test_code.rs:80`
  (`rule:programs/memory-priority`).

### 4. the small helpers are written once

**Does:** give the hex encoder, the string-argument accessor, the callable block and the probe
branch one home each.

- [ ] `crates/nvs-stdlib/src/encoding.rs:1617`: one hex encoder, used by `random.rs:769`,
  `server.rs:153`, `queue.rs:252` and `json.rs:1391` in `crates/nvs-stdlib/src/`
  (`rule:programs/memory-priority`).
- [ ] `crates/nvs-stdlib/src/csrf.rs:306`: one string-argument accessor, used by
  `signed_cookie.rs:292`, `session.rs:1061`, `ratelimit.rs:713`, `log.rs:297`, `html.rs:574` and
  `metrics.rs:413` in `crates/nvs-stdlib/src/` (`rule:errors/helper-abi`).
- [ ] `crates/nvs-runtime/src/callable.rs:771`: one function for the block at `:832` and `:1006`
  (`rule:errors/helper-abi`).
- [ ] `crates/nvs-codegen/src/emit.rs:1556`: one function for the probe branch at `:1765` and
  `:3973` (`rule:errors/helper-abi`).

### 5. the server tests of nvs-cli share their support

**Does:** move the `Server` boot, its `Drop`, the address parse and `TempDir` into one support
module, and convert the test files that use them.

**Split:** an `Explore` subagent lists every test file under `crates/` that declares
`struct Server`, `struct TempDir`, `const BOOT` or `const POLL`, as `file.rs:NN`, before the first
edit. Writing subagents then convert disjoint files side by side, and the session builds once.

- [ ] `crates/nvs-test/src/lib.rs:1`: the support module, written from
  `crates/nvs-cli/tests/live_config.rs:60` (`rule:testing/feature-proofs`).
- [ ] `crates/nvs-cli/tests/live_config.rs:60`: this file and the others on the list use the
  module, largest first (`rule:testing/feature-proofs`).

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done, built and clippy-clean, with no test run. Lines over `crates/`: stage 1
+156 -313, stage 2 +179 -254. Sessions used: stage 1: 1, stage 2: 1, stage 3: 0, stage 4: 0,
stage 5: 0.

- Stage 1: `Lowering::open_loop_header` and `close_foreach_body` in
  `crates/nvs-ir/src/lower/control.rs`, and `Lowering::into_lowered` in `lower/mod.rs`.
- Stage 2: the six statement rows of `crates/nvs-stdlib/src/db/registry.rs` (`QUERY_ROW` to
  `STREAM_AS_ROW`) are one `const` each, listed by `CONNECTION` and `TRANSACTION`.
  `rowsOf` in `tools/nv/cmd/gaps.ts` gives such a row to every class that lists it. The playbook
  trap "a `Core\Db` statement member is declared twice" is deleted, because it is no longer true.

The sweep's `env-secret-proofs` finding names a retired check: `looper check --name` finds no
such check and no log exists. Nothing in the tree is changed for it.

Tests that could go red in stage 6: the `insta` snapshots under
`crates/nvs-ir/src/lower/snapshots/`, and `db::transaction`'s
`a_transaction_is_a_callable_and_transaction_is_a_queryable`, which compares the two rosters.

### Next

Stage 3, in up to two sessions. Each item is its own slice and its own commit.

- [ ] `crates/nvs-lsp/src/hints.rs:124`: one walker with a visitor, used by
  `crates/nvs-lsp/src/semantic.rs:572`.
- [ ] `crates/nvs-test/src/case.rs:509`: the section parser is public, and
  `crates/nvs-lsp/src/case.rs:315` calls it.
- [ ] `crates/nvs-types/src/deprecated.rs:438`: one namespace walk, used by
  `crates/nvs-types/src/retrieval.rs:238` and `crates/nvs-types/src/test_code.rs:80`.

### Backlog

- Declined before the run: joining the per-driver code of `crates/nvs-db/` (reset, begin, commit,
  the read loop). Each driver speaks another protocol, and a shared form would hide where they
  differ.
- Declined before the run: joining the repeated lists and set-up of `tools/nv/`. No block there
  reaches 14 identical lines, and goal `cheaper-pipeline` edits the same files.
