---
looper: 1
title: "A moved Core name says where it went, and the binary prints what changed"
summary: "E0303 and E0306 on a moved Core name name its new place and carry a fix that rewrites it, the release changelog has a Formatter section, nvs agent changes --since prints the embedded changelog, and the checks below pass."
context: [docs/rules/tooling/an-agent-asks-the-binary.md, docs/rules/packaging/the-versioned-surface-is-enumerated.md]
stages:
  - id: 1
    title: "a moved Core name says where it went"
    status: done
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-hir/src/hierarchy.rs, crates/nvs-hir/src/autoload.rs, crates/nvs-hir/src/imports.rs, crates/nvs-diagnostics/src/diagnostic.rs, crates/nvs-cli/src/main.rs, crates/nvs-lsp/src/diagnostics.rs]
    extra:
      context:
        rules: [core-api/a-moved-name-says-where-it-went, packaging/the-versioned-surface-is-enumerated]
  - id: 2
    title: "the release changelog has a Formatter section"
    status: done
    context: [tools/nv/cmd/release.ts, tools/nv/test/release.test.ts, docs/agent/conventions.md]
  - id: 3
    title: "`nvs agent changes --since` prints the changelog the binary carries"
    status: done
    context: [crates/nvs-cli/src/agent.rs, crates/nvs-cli/tests/agent.rs, CHANGELOG.md, .github/workflows/release.yml, tools/nv/cmd/release.ts, docs/reference/tools/50-agents.md]
    extra:
      context:
        rules: [tooling/an-agent-asks-the-binary, tooling/nvs-agent-changes-prints-the-changelog]
  - id: 4
    title: the goal is verified once, and every failure is fixed
    status: done
    gate: true
checks:
  - id: agent-changes-help
    stage: 3
    name: nvs agent changes exists
    run: [cargo, run, -q, --bin, nvs, --, agent, changes, --help]
    stdout_contains: ["--since"]
  - id: owners-closes
    stage: 3
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, changes-for-agents]
    reach: true
    floor: false
---

# A moved Core name says where it went, and the binary prints what changed

## Why

A program, or a coding agent, that upgrades Novis meets three walls. A `Core` name that moved gives
`E0303` or `E0306` with no hint: the qualified `Core\X` case says nothing about where it went
(`crates/nvs-hir/src/hierarchy.rs:651`, `crates/nvs-hir/src/hierarchy.rs:766`), and
`imports::candidates` (`crates/nvs-hir/src/imports.rs:171`) matches exact short names only. A release
that changes what `nvs fmt` writes lists that change under *Other* or *Internal*
(`tools/nv/cmd/release.ts:105`, `tools/nv/cmd/release.ts:392`), so nobody knows to reformat. And the
changelog is a file in the repository, so an agent working in a project has no way to ask its binary
what changed since the version the project was written for.

## Standing decisions

**The table of moved names is data beside the registry.** It is a new
`crates/nvs-stdlib/src/moved.rs`: one row per move, with the old name, the new name and the version
it moved in. The first row is `Core\Order` to `Core\Arr::Order`. A test reads the registry and fails
if a row's new name does not exist or its old name still does. Why: the registry is the source of
truth, so the table cannot name a place that is not there.

**The note is a pass over the finished diagnostics, like the autoload note, and it lives in
`nvs-types`.** `nvs-hir` cannot read `nvs-stdlib`, and `nvs-stdlib` reads `nvs-hir` only in its
tests, so the pass is `crates/nvs-types/src/moved.rs`, the crate that already reads both. It reads
each subject with `nvs_hir::hierarchy::undeclared_subject` and
`nvs_hir::resolve::unresolved_import_subject`, and adds a help line and a fix to each one whose name
is in the table. The CLI calls it beside the autoload note (`crates/nvs-cli/src/main.rs:1289`), and
the language server at the end of its analysis (`crates/nvs-lsp/src/document.rs:796`). Why: no new
dependency edge, and the table stays pure data.

**A `Core` class that does not exist is `E0303` in an expression too.** `Core\Order::Desc` was
`E0405` ("has no constant named"), so the pass would have read a third headline. Now
`report_unknown_member` (`crates/nvs-types/src/expr/members.rs:2668`) reports a `Core` class nothing
declares as `E0303`, through `nvs_hir::hierarchy::not_declared`, and a short name an unresolved `use`
brought in is not reported again. Why: the class is the mistake, one code covers all three positions,
and `rule:types/one-mistake-reports-once`.

**A `use` line's fix deletes the line and rewrites every short reference.** A class-scoped enum cannot
be imported (`E0212`), so the one rewrite that fits every move is the new name in full at each
reference. Its suggestions are one fix, in `--fix` and in the editor (`crates/nvs-lsp/src/actions.rs:243`).
An aliased or grouped `use` gets the help line alone.

**A move made before the first release has no version.** `Move::since` is `None` for it, and the help
line names no version. Why: no release exists yet, so any number would be a guess.

**The fix is safe.** The table names exactly one replacement, so the suggestion is a safe one
(`crates/nvs-diagnostics/src/diagnostic.rs:115`), and `nvs check --fix` applies it. It writes the new
name in the form its position takes.

**A row stays until 1.0.** Why: before 1.0 any release may move a `Core` name, and a project may skip
many releases, so it meets every move at once. A row costs a few bytes in the binary and nothing at
run time, and it is read only on a failed compile. At 1.0 the table is emptied, and the mechanism
stays for the next major version. The rule records this.

**The rule is `core-api/a-moved-name-says-where-it-went`, a new fragment.** Its Declined list carries
an alias that keeps the old name working (`rule:statements/nothing-gets-a-second-name`) and a guess
by edit distance across `Core` (a guess is not a fix that can be applied safely).

**A commit scoped `fmt` goes under Formatter.** The release changelog gets a fourth section,
Formatter, listed in full after Performance (`tools/nv/cmd/release.ts:105`). A commit whose scope is
`fmt`, of any type, goes there. A breaking `!` is still lifted to the top, as today. Why: the scope is
the one thing the tool can read, and a project then reformats in one deliberate commit after reading
the list. `docs/agent/conventions.md` § *A commit message* says that a commit that changes what
`nvs fmt` writes takes the scope `fmt`.

**`CHANGELOG.md` is embedded whole.** `nvs agent changes` reads it through `include_str!`, with no
compression. Why: simplicity. The file is a few KiB today and stays far below the binary's size. If
it ever passes 1 MiB, a later goal compresses it.

**The binary carries its own version's section.** The build job applies only the manifests today
(`.github/workflows/release.yml:292`), so a released binary would miss its own section. The build job
also prepends the section to `CHANGELOG.md` before it compiles, from the same notes the publish job
commits (`tools/nv/cmd/release.ts:255`, `.github/workflows/release.yml:416`). Why: the binary must
answer for the version it is.

**The command's shape.** `nvs agent changes --since <version>` prints every section newer than
`<version>`, newest first, as the file has them. With no `--since` it prints the section of the
running version. `--json` prints the same versions with a schema version, in the shape the other
`nvs agent` commands use (`crates/nvs-cli/src/agent.rs:133`). A version older than the oldest section
prints all of them. A version newer than the binary prints nothing and says which version the binary
is. A malformed version is a usage error, exit `2`. `nvs agent primer` names the command. Its help
text follows AGENTS.md § *Text an end user reads*. The rule is `tooling/nvs-agent-changes-prints-the-changelog`,
a new fragment.

**Lanes.** Stage 1 (`nvs-stdlib`, the note's two callers) is a lane of its own. Stages 2 and 3 share
`release.ts`, so 3 runs after 2. The goal is a side goal: it depends on no unfinished chain goal. It
shares one call in `crates/nvs-cli/src/main.rs` with `cli-housekeeping` and one call in
`crates/nvs-lsp/src/diagnostics.rs` with `editor-refactors`, each a one-line merge.

**Trade-offs.** Security does not change. Latency on the request path does not change: the note runs
only on a failed compile, and the changelog is read only by `nvs agent changes`. Memory: the table and
the changelog are static bytes in the binary, never per request. Simplicity: one table, one changelog
section and one subcommand. Usability: a moved name gives the fix, and an agent can ask its binary
what changed before it edits a project.

## Stages

### 1. a moved Core name says where it went

**Does:** `E0303` and `E0306` on a name in the table say where it moved, and carry a safe fix.

- [x] The rule fragment and its `designed` record (`core-api/a-moved-name-says-where-it-went`). `docs/rules/core-api/a-moved-name-says-where-it-went.md:1`, `data/rules/core-api/a-moved-name-says-where-it-went.json:1`
- [x] The table, with its first row, and the test that reads the registry. `crates/nvs-stdlib/src/moved.rs:1`, `crates/nvs-stdlib/src/registry.rs:702`
- [x] Which code a moved name gets in a `use`, a type and an expression, confirmed. `crates/nvs-hir/src/hierarchy.rs:651`, `crates/nvs-hir/src/resolve.rs:260`
- [x] The pass adds the help line and the safe fix. `crates/nvs-types/src/moved.rs:1`, `crates/nvs-hir/src/hierarchy.rs:706`
- [x] The CLI and the language server call it. `crates/nvs-cli/src/main.rs:1289`, `crates/nvs-lsp/src/document.rs:796`
- [x] A conformance case: a moved name in a `use`, a type and an expression, each with its note. `tests/conformance/lang/a-moved-core-name-says-where-it-went.nvst:1`
- [x] A `--fix` test: `nvs check --fix` rewrites each one, and the program then compiles. `crates/nvs-cli/tests/check_flags.rs:1`
- [x] The rule `shipped` with both in `guardedBy`. `data/rules/core-api/a-moved-name-says-where-it-went.json:1`

### 2. the release changelog has a Formatter section

**Does:** `bun nv release` lists every commit scoped `fmt` under Formatter.

- [x] Formatter after Performance, listed in full, for any type scoped `fmt`. `tools/nv/cmd/release.ts:105`, `tools/nv/cmd/release.ts:392`
- [x] The module doc's section list names Formatter. `tools/nv/cmd/release.ts:51`
- [x] Tests: a `fix(fmt)` and a `refactor(fmt)` land under Formatter, a breaking `feat(fmt)!` is lifted to the top, and the Internal count leaves them out. `tools/nv/test/release.test.ts:1`
- [x] `docs/agent/conventions.md` says a commit that changes what `nvs fmt` writes takes the scope `fmt`. `docs/agent/conventions.md:26`

### 3. `nvs agent changes --since` prints the changelog the binary carries

**Does:** the binary embeds `CHANGELOG.md`, and a new `nvs agent` subcommand prints the sections
newer than a version.

- [x] The rule fragment and its `designed` record (`tooling/nvs-agent-changes-prints-the-changelog`). `docs/rules/tooling/nvs-agent-changes-prints-the-changelog.md:1`, `data/rules/tooling/nvs-agent-changes-prints-the-changelog.json:1`
- [x] The `Changes` variant, its dispatch, and the embedded file. `crates/nvs-cli/src/agent.rs:196`, `crates/nvs-cli/src/agent.rs:310`
- [x] The section parser, `--since`, the default and `--json`, as § *Standing decisions* says. `crates/nvs-cli/src/agent.rs:1`
- [x] The help text in the binary. `crates/nvs-cli/src/agent.rs:196`
- [x] `nvs agent primer` names the command. `crates/nvs-cli/src/agent.rs:817`
- [x] The build job prepends the version's section before it compiles. `.github/workflows/release.yml:294`, `tools/nv/cmd/release.ts:733`
- [x] `# nvs agent changes` in the reference. `docs/reference/tools/50-agents.md:106`
- [x] Tests over a changelog with three sections: `--since` each, no `--since`, an older version, a newer version, a malformed one, and `--json`. `crates/nvs-cli/tests/agent.rs:1`
- [x] The rule `shipped`, and `bun nv owners --closes changes-for-agents` passes. `data/rules/tooling/nvs-agent-changes-prints-the-changelog.json:1`

### 4. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [x] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [x] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [x] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Every stage is done. Both rules are shipped: `core-api/a-moved-name-says-where-it-went` and
`tooling/nvs-agent-changes-prints-the-changelog`. Stage 4 is done, and the goal is reached. `bun nv verify` was green on the first run. `bun nv ci
--check` was red on two counts. First, `tools:agents/nvs-agent-changes` owed its proofs. It now has
`about.md`, a `.cmd` example whose `.out` elides everything but `schemaVersion`, and a code block
in its reference section. A hostile skip in `data/proofs/skip/` matches `nvs agent init`'s, because the
command reads no program. Second, the site questions for the notes and paperwork guides were stale
after bf667cdeb's port change. They still fit, so they are restamped. The second verify,
`bun nv verify --doc`, `bun nv owners --closes changes-for-agents` and `bun nv ci --check` are all
green.

No tests are expected to go red.

The sweep's `env-secret-proofs` passes (exit 0). Run 40662443492 was a manual dispatch on 97fcf96ce.
Every job in it was cancelled, and none failed.

### Next

Nothing. The goal is reached, and the driver retires it. `docs/goals/README.md:1`

### Backlog
