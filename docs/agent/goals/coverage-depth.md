---
milestone: post-parity
position: last
---
# Loop goal 186 — coverage reports carry functions, branches and per-test lines, in Cobertura too, and VS Code shows them

`nvs test --coverage-lcov <FILE>` and `--coverage-clover <FILE>` write line coverage
(`rule:testing/coverage-report`, ADR 0235). This goal adds what CI services and editors read beside
lines:

```text
nvs test tests/ --coverage-lcov coverage.lcov --coverage-cobertura coverage.xml

coverage.lcov   SF / FN, FNDA / DA / BRDA, BRF, BRH / LF, LH        functions, lines, branches
coverage.xml    <coverage line-rate branch-rate> <class> <line branch="true" condition-coverage>
results.json    --format json: each test record gains "coverage": {"<file>": [lines]}
VS Code         a "Coverage" run profile in the Test Explorer, shown in the editor's own coverage view
```

## Why here

The user asked for coverage reports for CI, and chose to ship line coverage at once and to put
everything else in one goal at the very end of the chain, behind goal `growth-proof`. It carries
`position: last` for that reason. Nothing ahead of it depends on it, and it changes no language
behaviour.

What is built: the statement probe and the program-wide statement numbers
(`crates/nvs-codegen/src/emit.rs:@emit_stmt_probe`, `nvs_ir::Program::stmt_spans`), the shared
`StmtHits` table every context of a run counts into (`crates/nvs-runtime/src/ctx/trace.rs:@StmtHits`),
and the report writer (`crates/nvs-cli/src/coverage.rs`). What is not: the conditional-edge probe.
`nvs_ir::Terminator::Branch` already carries a `then_edge` and an `else_edge`
(`crates/nvs-ir/src/ir.rs:3108`), and `DebugFlags::BRANCH` is reserved
(`crates/nvs-runtime/src/ctx/mod.rs:@DebugFlags`), but codegen emits nothing for them
(`crates/nvs-codegen/src/emit.rs:3619`).

## Stage 0 — the catch-up

The sentences on disk this goal makes incomplete, each rewritten whole by the session that lands the
behaviour:

- `docs/rules/testing/coverage-report.md` — lines only; it gains functions, branches, Cobertura and
  the per-test key, one sentence each.
- `docs/rules/testing/report-formats.md` — "per-test coverage" is named as a key the JSON schema will
  carry; Stage 4 makes it true and the fragment says which version.
- `docs/reference/tools/10-cli.md` § *nvs test* — the coverage bullet and the `--format json` bullet.
- `crates/nvs-runtime/src/ctx/trace.rs:@nvs_probe_stmt`'s doc ("`BRANCH` needs the per-edge probe
  site that lands with `nvs_ir::Terminator::Branch`'s lowering").
- `docs/agent/goals/editor-surfaces.md` is a finished goal's prose and is not edited.

## Stage 1 — the floor

Goal `growth-proof`'s whole acceptance list, carried in by the goal switch. Never traded. A run
with no coverage flag compiles and runs exactly what it ran before this goal, and its output does not
change by a byte.

## Stage 2 — functions and Cobertura

**Does:** Adds function coverage to the lcov and Clover files, and a third file,
`--coverage-cobertura <FILE>`.

One file set: `crates/nvs-cli/src/coverage.rs`, `crates/nvs-cli/src/runner.rs`,
`crates/nvs-cli/src/main.rs`, `crates/nvs-cli/tests/test_coverage.rs`.

- **The decision record**, written first, for the whole goal. It `modifies`
  `testing/coverage-report` and states the tradeoffs from § *Standing decisions*.
- **A function is a `nvs_ir::Function`.** Its name is the IR's `Class::method`, its line the line of
  its first statement, and its count the count of that statement. A function with no statement is not
  listed. lcov gains `FN`, `FNDA`, `FNF` and `FNH`; Clover gains `<line type="method">` and the
  `methods`/`coveredmethods` metrics. The table of functions comes from the same walk as
  `Program::stmt_spans`, read before the program is dropped.
- **Cobertura XML** under `--coverage-cobertura <FILE>`: one `<package>` per directory, one `<class>`
  per file, `<line number hits>` per line, `line-rate` at every level. The same `Sites` table feeds
  all three writers.
- **Pinned by** the Stage 2 checks.

## Stage 3 — branches

**Does:** Emits the branch probe on every conditional edge, and adds branch counts to all three
files.

Two file sets, in this order. The probe: `crates/nvs-codegen/src/emit.rs`,
`crates/nvs-codegen/src/lib.rs`, `crates/nvs-runtime/src/ctx/trace.rs`,
`crates/nvs-codegen/tests/probes.rs`. The report: Stage 2's file set.

- **The probe** is `rule:testing/debug-probes`'s second site, in the statement probe's shape: one
  load of the flag word and one predicted-not-taken branch on each edge of a
  `Terminator::Branch`, and an out-of-line `nvs_probe_edge(ctx, edge)` under `DebugFlags::BRANCH`.
  Edge numbers get a per-function base exactly as statement numbers do, and
  `Program::edge_spans` lists them in that order. The counts go into a second table beside
  `StmtHits`, handed down the same way.
- **A branch is one `Terminator::Branch`**: its two edges are the two sides. lcov writes
  `BRDA:<line>,<block>,<side>,<count>` with `-` for a side whose branch never ran, and `BRF`/`BRH`.
  Clover writes `<line type="cond" truecount falsecount>` and the `conditionals` metrics. Cobertura
  writes `branch="true"` and `condition-coverage` on the line, and `branch-rate`.
- **The flags** turn both bits on. There is no flag for lines without branches.
- **Pinned by** the Stage 3 checks. The probe test counts both edges of an `if`, a `while`'s exit and
  re-entry, and a `match` arm, and asserts a run with no flag calls `nvs_probe_edge` zero times.

## Stage 4 — per-test lines

**Does:** Lists the lines each test reached in the `--format json` document, at `schemaVersion: 3`.

One file set: `crates/nvs-cli/src/runner.rs`, `crates/nvs-cli/src/coverage.rs`,
`crates/nvs-cli/tests/test_coverage.rs`, `crates/nvs-cli/tests/test_command.rs`.

- **Only with a coverage flag.** A run without one writes the version 2 document it writes today. A
  run with one writes version 3, where each `tests[]` record has a `coverage` object: file name to the
  sorted list of lines that test reached, at least once in any attempt.
- **The lines of one test** are the table's counts after the test minus the counts before it. The
  runner runs one test at a time, so nothing else counts in between. M10's parallel runner would break
  that; the session that builds it gives each test a table of its own and adds them up.
- **Pinned by** the Stage 4 checks.

## Stage 5 — coverage in VS Code

**Does:** Adds a Coverage run profile to the Test Explorer that reads the lcov file into VS Code's
own coverage view.

One file set: `editors/vscode/src/tests.ts`, `editors/vscode/test/surfaces/tests.test.ts`.

- `controller.createRunProfile(..., TestRunProfileKind.Coverage, ...)` beside the existing Run
  profile (`editors/vscode/src/tests.ts:77`). It runs the same argv
  (`editors/vscode/src/tests.ts:181`) with `--coverage-lcov` pointing into the extension's storage
  directory, reads the file, and adds one `FileCoverage` per `SF` record with its statement, branch
  and function counts, and `loadDetailedCoverage` returns its lines and branches.
- **No gutter, no decoration and no panel of the extension's own**
  (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
- **Pinned by** the Stage 5 check: a headless test feeds a fixed lcov file through the reader and
  checks the counts.

## Stage 6 — the reference and the proofs

**Does:** Rewrites the reference section, the help and `about.md` of `nvs test` for everything this
goal added.

No new reference heading, so no new feature on the roster: coverage is part of `tools:cli/nvs-test`.

- Every document Stage 0 lists, each rewritten whole.
- `docs/examples/tools/cli/nvs-test/about.md` says in one or two sentences that `nvs test` can write
  coverage files, inside its word band.
- `bun nv reference` regenerates `docs/novis.md`.

## Standing decisions

- **The user's calls.** Lines with lcov and Clover shipped first; this goal adds Cobertura, function
  and branch coverage, per-test coverage in the JSON report, and VS Code's coverage view, and nothing
  else. Choosing which files a report covers (an include or exclude list), an HTML report and
  merging reports from several runs are not in it; a session that needs one puts it in the handoff's
  `## Backlog` for the user.
- **No cost without a flag.** A run without a coverage flag allocates no table and its compiled code
  is what it is today plus the branch probe's load and branch, which `rule:testing/debug-probes`
  already prices. The bench that pins it is the statement probe's.
- **A count is a statement's or an edge's**, never a guess. A line or a branch the program has no
  probe for is not in a report.
- **Names follow ADR 0235**: relative to the run's directory with `/`, files not on disk left out, in
  every format.
- **One ADR slot**: one new record, checked right before it is written. It states the tradeoffs.
  Performance: one more load and branch per conditional edge in every unit, and a relaxed atomic add
  per edge taken under the flag. Memory: one `u64` per edge in the program, per run that asks.
  Usability: Cobertura is what GitLab and Azure DevOps read, branches are what reviewers ask for, and
  the editor shows the result. Simplicity: one more flag, one more probe site.
- **Every comment in a changed `.nvs` and every changed `about.md` follows `AGENTS.md` § *Text an end
  user reads* at the first write.**
