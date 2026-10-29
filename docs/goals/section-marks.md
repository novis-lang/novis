---
looper: 1
title: "Every section mark in the code names the document it cites"
summary: "No comment under crates/ or tests/conformance/ carries a bare section mark, every mark that cited a deleted decision record is rewritten to the rule or module doc that holds the fact, nvs rules --check refuses a bare mark, and the checks below pass."
context: [tools/nv/cmd/rules.ts, tools/nv/test/rules.test.ts]
stages:
  - id: 1
    title: "the check finds every bare mark, and no file gains one"
    status: done
    context: [tools/nv/cmd/rules.ts, tools/nv/test/rules.test.ts, docs/agent/conventions.md]
  - id: 2
    title: "the standard library's queue and database files' marks name their documents"
    status: done
    context: [tools/data/bare-section-marks.json, crates/nvs-stdlib/src/queue.rs, crates/nvs-stdlib/src/db/execute.rs]
  - id: 9
    title: "the rest of the standard library's marks name their documents"
    status: done
    context: [tools/data/bare-section-marks.json, crates/nvs-stdlib/src/uri.rs, crates/nvs-stdlib/src/request.rs, crates/nvs-stdlib/src/response.rs]
  - id: 3
    title: "the PostgreSQL, MySQL and SQLite drivers' marks name their documents"
    status: done
    context: [tools/data/bare-section-marks.json, crates/nvs-db/src/pg.rs, crates/nvs-db/src/mysql.rs, crates/nvs-db/src/sqlite.rs]
  - id: 10
    title: "the TDS driver and the rest of the database crate's marks name their documents"
    status: done
    context: [tools/data/bare-section-marks.json, crates/nvs-db/src/tds/plan.rs, crates/nvs-db/src/conn.rs]
  - id: 4
    title: "the type checker's, parser's and code generator's marks name their documents"
    status: open
    context: [tools/data/bare-section-marks.json, crates/nvs-types/src/routes.rs, crates/nvs-types/src/lib.rs]
  - id: 11
    title: "the IR, HIR and diagnostics crates' marks name their documents"
    status: open
    context: [tools/data/bare-section-marks.json, crates/nvs-ir/src/lib.rs, crates/nvs-diagnostics/src/lib.rs]
  - id: 5
    title: "the server and host crates' marks name their documents"
    status: open
    context: [tools/data/bare-section-marks.json, crates/nvs-server/src/lib.rs, crates/nvs-host/src/lib.rs]
  - id: 12
    title: "the runtime crate's marks name their documents"
    status: open
    context: [tools/data/bare-section-marks.json, crates/nvs-runtime/src/lib.rs]
  - id: 6
    title: "the config, LDAP, test and render crates' marks name their documents"
    status: open
    context: [tools/data/bare-section-marks.json, crates/nvs-config/src/lib.rs]
  - id: 13
    title: "the CLI crate's marks name their documents"
    status: open
    context: [tools/data/bare-section-marks.json, crates/nvs-cli/src/cache.rs]
  - id: 7
    title: "the core conformance cases' marks name their documents"
    status: open
    context: [tools/data/bare-section-marks.json, tests/conformance/core/uri-resolve-runs-rfc-3986-s-5-4-abnormal-table.nvst]
  - id: 14
    title: "the other conformance cases' marks name their documents"
    status: open
    context: [tools/data/bare-section-marks.json]
  - id: 8
    title: "the check refuses every bare mark"
    status: open
    context: [tools/nv/cmd/rules.ts]
  - id: 15
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: rules
    stage: 1
    name: "the rulebook is clean, and no file has more bare section marks than its recorded count"
    run: [bun, nv, rules, --check]
---

# Every section mark in the code names the document it cites

<!-- nv-rules:record-examples -->

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
  rises or a file with none gains one. A stage lowers the counts it reaches in the same commit, by
  `bun nv rules --record-marks`. Stage 8 deletes the record, and the check then refuses every bare
  mark.
- **The record drives every slice.** A stage's files are the entries of
  `tools/data/bare-section-marks.json` under the paths its items name, taken largest count first until
  about 600 bare marks, which is what each stage was sized to from a raw count of `§ N`. Where a
  stage's entries total more, the session stops at about 600 and its handoff names the files left as
  the next group.
- **The history lookup and the edits are delegated to the same writing subagent.** Each gets a
  disjoint set of files, runs the history method above, edits the comments, and reports one line per
  mark meaning: the record and section, and the replacement. They run side by side, and the session
  reads every diff before it commits. A separate `Explore` table first would read each file twice
  and check nothing the diff does not. `bun nv rules --marks <paths>` lists what is left in a file.
  A slice is a set of files whose diff the session read, committed with the counts it lowered.
- **A mark in a string literal is rewritten by the session, after the subagents end.** Subagents
  edit comments only and report each mark left in a string. Most are test messages, which name the
  thing instead (`"§ 6 moved it"` becomes `"the dead-letter move took it"`). A message a program's
  user reads follows AGENTS.md § *Text an end user reads*, and is changed only once a search shows
  no expectation file pins its wording.

**Lanes.** Stages 2 to 7 and 9 to 14 cover disjoint crates and directories, so any two can run at
once as lanes after stage 1. Every lane edits `tools/data/bare-section-marks.json`; at merge it is
regenerated with `bun nv rules --record-marks`, never merged by hand. A lane avoids a crate another
goal is editing: stages 2, 9, 4, 11, 12 and 6 touch files goal `namespace-grants` edits
(`crates/nvs-stdlib/src/cap.rs`, `crates/nvs-stdlib/src/registry.rs`, `crates/nvs-types/src/capability.rs`,
`crates/nvs-hir/src/requires.rs`, `crates/nvs-diagnostics/src/code/config.rs`,
`crates/nvs-runtime/src/capability.rs`, `crates/nvs-config/src/resolve.rs`), and stage 13 touches
`crates/nvs-cli/`, which every package goal edits. Such a stage waits until that goal's file is gone
from `docs/goals/`, or skips the shared files and leaves them for its handoff.

**Gates.** Every stage after the first needs `tools/data/bare-section-marks.json` (stage 1). Stage 8
needs every other stage done. When a crate stage's shared files are held by a live goal, the session
takes another crate stage, named in the lanes above.

**Trade-offs.** No program, message or result changes: only comments, conformance case
descriptions and one check. No memory and no time on the request path. Simplicity of the
implementation gains, because every citation in the code leads to a text that exists. The cost is
sessions: fourteen stages, each about one session.

## Stages

### 1. the check finds every bare mark, and no file gains one

**Does:** `bun nv rules --check` counts the bare marks in every tracked file under `crates/` and
`tests/conformance/`, and fails when a file has more than its recorded count.

- [x] `conventions.md` § *Citing a document* states the three forms a section mark takes. `docs/agent/conventions.md:367`
- [x] A bare-mark finder beside `deletedDocFindings`, with cases for each kept form and for a bare mark. `tools/nv/cmd/rules.ts:120`, `tools/nv/test/rules.test.ts:29`
- [x] The per-file counts are recorded in `tools/data/bare-section-marks.json`, which `bun nv rules --record-marks` writes and the check reads. `tools/nv/cmd/rules.ts:144`, `tools/nv/cmd/rules.ts:467`
- [x] The check fails on a count that rose and on a new file with a mark, and passes on a count that fell. `tools/nv/test/rules.test.ts:46`

### 2. the standard library's queue and database files' marks name their documents

**Does:** no queue or database file under `crates/nvs-stdlib/` has a bare mark.

**Split:** one `Explore` subagent per file returns its mark table; writing subagents apply the tables
in batches, one after another, as the standing decision says.

- [x] The queue files. `crates/nvs-stdlib/src/queue.rs:1`, `crates/nvs-stdlib/tests/queue.rs:1`, `crates/nvs-stdlib/tests/queue_sqlite.rs:1`, `crates/nvs-stdlib/src/topic.rs:1`, `crates/nvs-stdlib/src/bus.rs:1`
- [x] The database layer: every file under `crates/nvs-stdlib/src/db/` the record lists, and `crates/nvs-stdlib/tests/db_stream.rs`. `crates/nvs-stdlib/src/db/execute.rs:1`, `crates/nvs-stdlib/src/db/transaction.rs:1`
- [x] Their counts at zero in the record. `tools/data/bare-section-marks.json:1`

### 9. the rest of the standard library's marks name their documents

**Does:** no file under `crates/nvs-stdlib/` has a bare mark.

**Split:** as stage 2.

- [x] The HTTP, URI and request files: `uri.rs`, `request.rs`, `response.rs`, `socket.rs`, `http.rs`, `router.rs`, `sse.rs` and `http/`. `crates/nvs-stdlib/src/uri.rs:1`, `crates/nvs-stdlib/src/request.rs:18`, `crates/nvs-stdlib/src/http.rs:5`
- [x] The rest of the crate's entries in the record, largest first: `ratelimit.rs`, `cache.rs`, `log.rs`, `script.rs`, `encoding.rs`, `crypto.rs`, `command.rs` and the files below them. `crates/nvs-stdlib/src/ratelimit.rs:1`
- [x] The crate's counts at zero. `tools/data/bare-section-marks.json:1`

### 3. the PostgreSQL, MySQL and SQLite drivers' marks name their documents

**Does:** the three drivers have no bare mark. Many of these cite a wire protocol's documentation,
which keeps its number and gains its name.

**Split:** as stage 2.

- [x] The PostgreSQL driver. `crates/nvs-db/src/pg.rs:1`
- [x] The MySQL and MariaDB drivers, and the handshake tests. `crates/nvs-db/src/mysql.rs:1`, `crates/nvs-db/src/maria.rs:1`, `crates/nvs-db/tests/handshake.rs:1`
- [x] The SQLite driver, and these files' counts at zero. `crates/nvs-db/src/sqlite.rs:1`

### 10. the TDS driver and the rest of the database crate's marks name their documents

**Does:** no file under `crates/nvs-db/` has a bare mark.

**Split:** as stage 2.

- [x] Every file under `crates/nvs-db/src/tds/`. `crates/nvs-db/src/tds/plan.rs:1`, `crates/nvs-db/src/tds/rpc.rs:1`
- [x] `conn.rs`, `sql.rs`, `lib.rs`, `span.rs`, `direct.rs`, `ddl.rs` and `tests/pool_reuse.rs`. `crates/nvs-db/src/conn.rs:1`
- [x] The crate's counts at zero. `tools/data/bare-section-marks.json:1`

### 4. the type checker's, parser's and code generator's marks name their documents

**Does:** no file under `nvs-types`, `nvs-syntax` or `nvs-codegen` has a bare mark.

**Split:** as stage 2.

- [ ] `nvs-types`'s routes, intrinsics and expression files. `crates/nvs-types/src/routes.rs:1`, `crates/nvs-types/tests/routes.rs:1`, `crates/nvs-types/src/expr/operators.rs:1`
- [ ] The rest of `nvs-types`'s entries in the record. `crates/nvs-types/src/lib.rs:1`
- [ ] `nvs-syntax` and `nvs-codegen`, and the three crates' counts at zero. `crates/nvs-syntax/src/lib.rs:1`, `crates/nvs-codegen/src/lib.rs:1`

### 11. the IR, HIR and diagnostics crates' marks name their documents

**Does:** no file under `nvs-ir`, `nvs-hir` or `nvs-diagnostics` has a bare mark.

**Split:** as stage 2.

- [ ] `nvs-ir`. `crates/nvs-ir/src/ir.rs:1`, `crates/nvs-ir/src/lower/convert.rs:1`
- [ ] `nvs-hir`. `crates/nvs-hir/src/lib.rs:1`
- [ ] `nvs-diagnostics`, and the three crates' counts at zero. `crates/nvs-diagnostics/src/lib.rs:1`

### 5. the server and host crates' marks name their documents

**Does:** no file under `nvs-server` or `nvs-host` has a bare mark.

**Split:** as stage 2.

- [ ] `nvs-server`'s schedule, serve and mount files. `crates/nvs-server/src/schedule.rs:1`, `crates/nvs-server/src/serve.rs:1`, `crates/nvs-server/src/mount.rs:1`
- [ ] The rest of `nvs-server`'s entries in the record. `crates/nvs-server/src/lib.rs:1`
- [ ] `nvs-host`, and both crates' counts at zero. `crates/nvs-host/src/lib.rs:1`

### 12. the runtime crate's marks name their documents

**Does:** no file under `nvs-runtime` has a bare mark.

**Split:** as stage 2.

- [ ] The capability, routes, metrics and pool files. `crates/nvs-runtime/src/capability.rs:1`, `crates/nvs-runtime/src/routes.rs:1`
- [ ] The files under `crates/nvs-runtime/src/ctx/`. `crates/nvs-runtime/src/ctx/mod.rs:1`
- [ ] The rest of the crate's entries in the record, and its counts at zero. `crates/nvs-runtime/src/lib.rs:1`

### 6. the config, LDAP, test and render crates' marks name their documents

**Does:** no file under `nvs-config`, `nvs-ldap`, `nvs-test` or `nvs-render` has a bare mark.

**Split:** as stage 2.

- [ ] `nvs-config`. `crates/nvs-config/src/http.rs:1`, `crates/nvs-config/src/lib.rs:1`
- [ ] `nvs-ldap`, `nvs-test` and `nvs-render`, and the four crates' counts at zero. `crates/nvs-ldap/src/lib.rs:1`

### 13. the CLI crate's marks name their documents

**Does:** no file under `crates/` has a bare mark.

**Split:** as stage 2.

- [ ] `nvs-cli`'s cache, worker and serve files. `crates/nvs-cli/src/cache.rs:27`, `crates/nvs-cli/src/worker.rs:1`, `crates/nvs-cli/src/serve.rs:1`
- [ ] The rest of `nvs-cli`'s entries in the record, and every count under `crates/` at zero. `crates/nvs-cli/src/main.rs:1`

### 7. the core conformance cases' marks name their documents

**Does:** no case under `tests/conformance/core/` has a bare mark.

**Split:** as stage 2, with one `Explore` subagent per batch of about twenty cases, since each case is
short.

- [ ] The cases whose names start `a` to `d`. `tests/conformance/core/uri-resolve-runs-rfc-3986-s-5-4-abnormal-table.nvst:1`
- [ ] The cases from `e` on, and the directory's counts at zero. `tools/data/bare-section-marks.json:1`

### 14. the other conformance cases' marks name their documents

**Does:** no case under `tests/conformance/` has a bare mark.

**Split:** as stage 7.

- [ ] `tests/conformance/lang/`. `tools/data/bare-section-marks.json:1`
- [ ] `reject/`, `class/`, `error/` and `config/`, and every count at zero. `tools/data/bare-section-marks.json:1`

### 8. the check refuses every bare mark

**Does:** the record of counts is gone, and `rules --check` fails on any bare mark under `crates/`
or `tests/conformance/`.

- [ ] `tools/data/bare-section-marks.json` and `--record-marks` are deleted, and the check refuses every bare mark. `tools/nv/cmd/rules.ts:120`
- [ ] The cases in `rules.test.ts` pin the refusal. `tools/nv/test/rules.test.ts:29`

### 15. the goal is verified once, and every failure is fixed

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

Stages 1, 2, 9, 3 and 10 are done, and no file under `crates/nvs-db/` has a bare mark. The
record holds 3112 marks in 627 files. Two commits landed stage 10's 326 marks: `d7ebab43b` (the
TDS driver) and `5d6754029` (`conn.rs`, `sql.rs`, `lib.rs`, `span.rs`, `direct.rs`, `ddl.rs` and
`tests/pool_reuse.rs`). Four writing subagents on disjoint files did the comments. The session
rewrote the 36 marks left in strings and read every diff. Build, clippy and rustfmt are green.

Every stage 10 mark cited the database record 0067, except one in a `ddl.rs` plan-step reason,
which cited the schema record 0145 § 6 and now says "this grades up". No expectation file pins
that reason. The `[MS-TDS] §` marks were already named. One test was renamed:
`a_declarations_markers_are_the_ones_the_marker_rewriter_wrote` in `tds/rpc.rs`. Only comments,
test messages and that one reason string changed, so no test is expected to go red.

The subagents wrapped rewritten comments unevenly, and four came back past 100 columns. Check
`git diff -U0 | grep '^+' | awk 'length > 101'` before the build.

`bun nv rules --check` is red on one line this goal did not write:
`docs/goals/app-settings.md:151` cites a rule that goal will write, outside its front matter.
That goal's first session fixes it.

### Next

Stage 4, `nvs-types`, `nvs-syntax` and `nvs-codegen`: 605 marks in 68 files, about one
session. Goal `namespace-grants` is gone, so its shared files are free. These crates cite many
records, not just the database one, so each file's history decides the record: the method is in
Standing decisions. Split across four writing subagents on disjoint files, about 150 marks each,
and rewrite the marks left in strings after they end. `nvs-types/tests/` messages may be pinned
by nothing but the test itself; search before changing a diagnostic's text.
- [ ] `src/routes.rs` 68 and `tests/routes.rs` 59, with `src/links.rs` 18. `crates/nvs-types/src/routes.rs:1`
- [ ] `tests/intrinsics.rs` 39, `src/intrinsics.rs` 28, `src/expr_table.rs` 31, `tests/single_value_types.rs` 27, `tests/commands.rs` 23. `crates/nvs-types/src/intrinsics.rs:1`
- [ ] `src/expr/` (operators 33, quals 13, calls 12, members 11, literals 8 and the rest), `src/derive.rs` 17, `src/retrieval.rs` 12. `crates/nvs-types/src/expr/operators.rs:1`
- [ ] The rest of the three crates' entries in the record, from `crates/nvs-types/tests/classes.rs` 12 down, with `nvs-syntax` and `nvs-codegen`. `crates/nvs-codegen/src/lib.rs:1`
- [ ] `bun nv rules --record-marks` in each slice's commit. Stash the other slice's files while you record and commit one. `tools/data/bare-section-marks.json:1`

### Backlog

- A standard the finder does not know turns its marks bare; add it to `NAMED_STANDARD` with a case
  rather than rewriting the mark. `tools/nv/cmd/rules.ts:150`
- `Core\Db\Schema::fromArray`'s error shows `rule:core-classes/schema-vocabulary-is-closed` to the
  program's user. It is not a mark, and a conformance case pins it, so it changes with its
  expectation file. `crates/nvs-stdlib/src/db/schema.rs:58`
- `transaction.rs` and `uri.rs` cite "R2", "R5", "R9", "R10" and "R17", old numbered `Core` API
  conventions. They are not marks, so stage 8's check does not see them.
  `crates/nvs-stdlib/src/db/transaction.rs:745`, `crates/nvs-stdlib/src/uri.rs:23`
- `nvs_runtime::pool`'s head comment has bare § 13 marks, which `db/registry.rs` now cites by
  heading. Stage 12 rewrites them. `crates/nvs-runtime/src/pool.rs:4`
- A `§ N` in a doc under `docs/` is outside this goal. `bun nv verify --doc` checks links, not
  marks. `docs/agent/conventions.md:367`
