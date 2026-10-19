---
looper: 1
title: "Every section mark in the code names the document it cites"
summary: "No comment under crates/ or tests/conformance/ carries a bare section mark, every mark that cited a deleted decision record is rewritten to the rule or module doc that holds the fact, nvs rules --check refuses a bare mark, and the checks below pass."
context: [tools/nv/cmd/rules.ts, tools/nv/test/rules.test.ts]
stages:
  - id: 1
    title: "the check finds every bare mark, and no file gains one"
    status: open
    context: [tools/nv/cmd/rules.ts, tools/nv/test/rules.test.ts]
  - id: 2
    title: "the standard library's marks name their documents"
    status: open
    context: [crates/nvs-stdlib/src/queue.rs, crates/nvs-stdlib/src/db/execute.rs, crates/nvs-stdlib/src/uri.rs]
  - id: 3
    title: "the database drivers' marks name their documents"
    status: open
    context: [crates/nvs-db/src/pg.rs, crates/nvs-db/src/mysql.rs, crates/nvs-db/src/sqlite.rs]
  - id: 4
    title: "the compiler crates' marks name their documents"
    status: open
    context: [crates/nvs-types/src/lib.rs, crates/nvs-ir/src/lib.rs, crates/nvs-diagnostics/src/lib.rs]
  - id: 5
    title: "the server, runtime and host crates' marks name their documents"
    status: open
    context: [crates/nvs-server/src/lib.rs, crates/nvs-runtime/src/lib.rs, crates/nvs-host/src/lib.rs]
  - id: 6
    title: "the config, CLI and remaining crates' marks name their documents"
    status: open
    context: [crates/nvs-config/src/lib.rs, crates/nvs-cli/src/cache.rs]
  - id: 7
    title: "the conformance cases' marks name their documents"
    status: open
    context: [tests/conformance/core/uri-resolve-runs-rfc-3986-s-5-4-abnormal-table.nvst]
  - id: 8
    title: "the check refuses every bare mark"
    status: open
    context: [tools/nv/cmd/rules.ts]
checks:
  - id: rules
    stage: 1
    name: "the rulebook is clean, and no file has more bare section marks than its recorded count"
    run: [bun, nv, rules, --check]
---

# Every section mark in the code names the document it cites

## Why

About 5,300 lines under `crates/` and `tests/conformance/` carry a `§ N` mark that names no document.
Most of them once cited a section of a decision record. A module's head comment named the record
once (`[ADR 0042]'s on-disk artifact cache`), and every later `§ 1` or `§ 4` in the file meant that
record's section. When the records were merged into the rules, each record id was replaced by the
rule that took it over (commit `a2e6020bc` is one such), and the marks stayed. They now point at
sections of a rule that has no numbered sections. `rules --check` refuses a record id and a deleted
doc by path, but a bare `§ N` names no file, so it cannot tell a dead mark from an RFC's.

A comment says what the code does now, and a mark that points at nothing makes the reader search
for a document that does not exist.

## Standing decisions

**The user decided:** this is its own side goal, off the chain, run in a lane when one is free.

**The agent's calls:**
- **A mark that is kept names its document in the same phrase:** `RFC 3986 § 5.4`, `[MS-TDS] §
  2.2.6`, or `§ *Heading*` for a section of a document in this repository, cited by its heading.
  Any other `§ N` is bare. Stage 1 writes these forms into [conventions.md](../agent/conventions.md)
  § *Citing a document*, which is their home from then on.
- **A dead mark is rewritten from what its record said, never guessed.** The file's history names
  the record: `git log -p -S'ADR ' -- <file>` finds the commit that replaced its id, whose parent
  has it. The record is `docs/adr/<NNNN>-*.md` before `8b0d14919`, or `docs/decisions/<NNNN>.md`
  before `1be573200`, read with `git show <commit>:<path>`. The section's fact is then cited by the
  rule that holds it now (`rule:<topic>/<rule>`), usually the one the line already names. Where the
  sentence says the fact itself, the mark is deleted. Where no rule or module doc holds a fact that
  is still true, the fact is written into the module doc, and the commit says so.
- **A mark that cites a standard keeps its number and gains the standard's name,** even where the
  head comment already named it: `§ 3's TLS` becomes `RFC 8446 § 3's TLS`.
- **Stage 1 records each file's count of bare marks,** and the check fails when a file's count
  rises or a file with none gains one. A stage lowers the counts it reaches in the same commit.
  Stage 8 deletes the record, and the check then refuses every bare mark.
- **One crate group per session at most.** A slice is a set of files the session read in full,
  committed with the counts it lowered.

**Trade-offs.** No program, message or result changes: only comments, conformance case
descriptions and one check. No memory and no time on the request path. Simplicity of the
implementation gains, because every citation in the code leads to a text that exists. The cost is
sessions: the larger crates take several each.

## Stages

### 1. the check finds every bare mark, and no file gains one

**Does:** `bun nv rules --check` counts the bare marks in every tracked file under `crates/` and
`tests/conformance/`, and fails when a file has more than its recorded count.

- [ ] `conventions.md` § *Citing a document* states the three forms a section mark takes. `docs/agent/conventions.md:363`
- [ ] A bare-mark finder beside `deletedDocFindings`, with cases for each kept form and for a bare mark. `tools/nv/cmd/rules.ts:118`, `tools/nv/test/rules.test.ts:44`
- [ ] The per-file counts are recorded in `tools/data/bare-section-marks.json`, which `bun nv rules --record-marks` writes and the check reads. `tools/nv/cmd/rules.ts:146`
- [ ] The check fails on a count that rose and on a new file with a mark, and passes on a count that fell. `tools/nv/test/rules.test.ts:44`

### 2. the standard library's marks name their documents

**Does:** no file under `crates/nvs-stdlib/` has a bare mark.

- [ ] The queue files. `crates/nvs-stdlib/src/queue.rs:1`, `crates/nvs-stdlib/tests/queue.rs:1`
- [ ] The database layer. `crates/nvs-stdlib/src/db/execute.rs:1`, `crates/nvs-stdlib/src/db/transaction.rs:1`
- [ ] The HTTP, URI and request files. `crates/nvs-stdlib/src/uri.rs:1`, `crates/nvs-stdlib/src/request.rs:18`, `crates/nvs-stdlib/src/http.rs:5`
- [ ] Every other file in the crate, and its counts at zero.

### 3. the database drivers' marks name their documents

**Does:** no file under `crates/nvs-db/` has a bare mark. Many of these cite a wire protocol's
documentation, which keeps its number and gains its name.

- [ ] The PostgreSQL driver. `crates/nvs-db/src/pg.rs:1`
- [ ] The MySQL driver. `crates/nvs-db/src/mysql.rs:1`
- [ ] The SQLite driver and the rest of the crate, and its counts at zero. `crates/nvs-db/src/sqlite.rs:1`

### 4. the compiler crates' marks name their documents

**Does:** no file under `nvs-syntax`, `nvs-hir`, `nvs-types`, `nvs-ir`, `nvs-codegen` or
`nvs-diagnostics` has a bare mark.

- [ ] `nvs-types`. `crates/nvs-types/src/lib.rs:1`
- [ ] `nvs-ir`, `nvs-hir`, `nvs-syntax` and `nvs-codegen`. `crates/nvs-ir/src/lib.rs:1`
- [ ] `nvs-diagnostics`, and the group's counts at zero. `crates/nvs-diagnostics/src/lib.rs:1`

### 5. the server, runtime and host crates' marks name their documents

**Does:** no file under `nvs-server`, `nvs-runtime` or `nvs-host` has a bare mark.

- [ ] `nvs-server`. `crates/nvs-server/src/lib.rs:1`
- [ ] `nvs-runtime`. `crates/nvs-runtime/src/lib.rs:1`
- [ ] `nvs-host`, and the group's counts at zero. `crates/nvs-host/src/lib.rs:1`

### 6. the config, CLI and remaining crates' marks name their documents

**Does:** no file under `crates/` has a bare mark.

- [ ] `nvs-config`. `crates/nvs-config/src/lib.rs:1`
- [ ] `nvs-cli`. `crates/nvs-cli/src/cache.rs:27`
- [ ] `nvs-ldap`, `nvs-test`, `nvs-render` and every other crate, and the counts under `crates/` at zero.

### 7. the conformance cases' marks name their documents

**Does:** no case under `tests/conformance/` has a bare mark.

- [ ] `tests/conformance/core/`. `tests/conformance/core/uri-resolve-runs-rfc-3986-s-5-4-abnormal-table.nvst:1`
- [ ] `lang/`, `reject/`, `class/`, `error/` and `config/`, and the counts at zero.

### 8. the check refuses every bare mark

**Does:** the record of counts is gone, and `rules --check` fails on any bare mark under `crates/`
or `tests/conformance/`.

- [ ] `tools/data/bare-section-marks.json` and `--record-marks` are deleted, and the check refuses every bare mark. `tools/nv/cmd/rules.ts:118`
- [ ] The cases in `rules.test.ts` pin the refusal. `tools/nv/test/rules.test.ts:44`

## Handoff

### State

The goal is new. Nothing is built. The mark counts and the history method above were checked on
`side/docs-cleanup` when the goal was written.

### Next

Stage 1, the check, on `tools/nv/cmd/rules.ts` and `tools/nv/test/rules.test.ts`:
- [ ] The bare-mark finder and its cases. `tools/nv/cmd/rules.ts:118`
- [ ] The recorded counts and the check that holds them. `tools/nv/cmd/rules.ts:146`

Stage 2 follows in its own session, on `crates/nvs-stdlib/`.

### Backlog

- A `§ N` in a doc under `docs/` is outside this goal. `bun nv verify --doc` checks links, not
  marks. `docs/agent/conventions.md:363`
