---
looper: 1
title: "Trace events have times, and the profiler exports a timeline and a call graph"
summary: "Every trace event has a time on one clock, Core\\Debug streams a trace and a profile to a named sink, nvs run and nvs test write --profile=callgrind and --profile=speedscope, VS Code opens the speedscope file, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-stdlib/src/debug.rs, crates/nvs-cli/src/main.rs]
stages:
  - id: 1
    title: every trace event has a time on one clock
    status: done
    context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-server/src/trace.rs, crates/nvs-server/src/otlp.rs, docs/rules/observability/four-kinds-become-a-span.md]
    extra:
      context:
        rules: [observability/trace-events-carry-a-kind, observability/four-kinds-become-a-span, observability/gc-pause-is-its-own-event, observability/spawn-is-its-own-event]
  - id: 2
    title: "`Core\\Debug` starts and stops each probe"
    status: done
    context: [crates/nvs-stdlib/src/debug.rs, crates/nvs-config/src/capability.rs, crates/nvs-runtime/src/ctx/mod.rs, docs/rules/testing/debug-surface.md, docs/rules/core-api/no-mode-strings.md]
    extra:
      context:
        rules: [testing/debug-surface, testing/debug-probes, testing/debug-mode-directive, core-api/no-mode-strings, security/secret-qualifier]
  - id: 6
    title: a trace streams to a named sink
    status: done
    context: [crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-stdlib/src/debug.rs, docs/rules/testing/debug-surface.md]
    extra:
      context:
        rules: [testing/debug-surface, testing/debug-probes]
  - id: 7
    title: "the new `Core\\Debug` members have their feature proofs"
    status: open
    context: [crates/nvs-stdlib/src/debug.rs, docs/rules/testing/feature-proofs.md, docs/rules/core-api/reference-card.md, docs/examples/core/Debug/dump/about.md]
    extra:
      context:
        rules: [testing/feature-proofs, core-api/reference-card, testing/debug-surface]
  - id: 3
    title: a spawned isolate's probe data comes back on its result
    status: open
    context: [crates/nvs-host/src/group.rs, crates/nvs-host/src/placed.rs, crates/nvs-runtime/src/ctx/trace.rs, docs/rules/testing/debug-surface.md]
    extra:
      context:
        rules: [testing/debug-surface, security/isolate-shares-nothing, observability/spawn-is-its-own-event]
  - id: 4
    title: "the profiler times every call, and `--profile` writes Callgrind or speedscope"
    status: open
    context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-cli/src/run.rs, crates/nvs-cli/src/runner.rs, docs/rules/observability/speedscope-timeline-export.md, docs/rules/testing/debug-surface.md]
    extra:
      context:
        rules: [observability/speedscope-timeline-export, testing/debug-surface, testing/debug-probes, testing/report-formats]
  - id: 5
    title: VS Code opens a profile
    status: open
    context: [editors/vscode/src/extension.ts, editors/vscode/package.json, docs/rules/ide/the-extension-builds-no-ui-the-editor-already-has.md]
    extra:
      context:
        rules: [ide/the-extension-builds-no-ui-the-editor-already-has, observability/speedscope-timeline-export]
  - id: 8
    title: the profiler has its feature proofs
    status: open
    context: [docs/reference/tools/10-cli.md, docs/rules/testing/feature-proofs.md, docs/rules/observability/speedscope-timeline-export.md, benches/members/README.md]
    extra:
      context:
        rules: [testing/feature-proofs, observability/speedscope-timeline-export, testing/member-perf-ledger]
  - id: 9
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: owners-closes
    stage: 8
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, trace-timeline]
    reach: true
    floor: false
---

# Trace events have times, and the profiler exports a timeline and a call graph

## Why

A trace event records what happened and not when. `TraceEvent` has a kind, a label and a status
(`crates/nvs-runtime/src/ctx/trace.rs:102`), so an OTLP span has no duration and no timeline can be
drawn. The call half of the
trace grows in a vector without bound (`crates/nvs-runtime/src/ctx/mod.rs:1381`), which is why
`TRACE` is off for every served request. The `PROFILE` bit exists and no call probe reads it
(`crates/nvs-runtime/src/ctx/trace.rs:769-803`). `Core\Debug` can dump and render a value and cannot
start or stop a probe (`crates/nvs-stdlib/src/debug.rs:117`). This goal gives every event a time,
streams the unbounded half to a sink, and builds the one profiler with its two exports.
`debug-viewer` reads the timeline this goal writes.

## Standing decisions

**One profiler, two exports.** The user decided this. Only the probe-based profiler is built:
`nvs run --profile=callgrind` and `--profile=speedscope`, and the same under `nvs test`. There is no
sampling profiler. Its docs say that the probe per call makes call-heavy code look slower. VS Code's
*View Profile* command opens the speedscope file. `rule:testing/debug-surface` and
`rule:observability/speedscope-timeline-export` hold it.

**One clock for every event.** Every event carries its start and end as readings of the one process
epoch, `EPOCH` (`crates/nvs-runtime/src/ctx/trace.rs:31`), read with `nvs_runtime::trace_clock`. One
wall-clock reading taken with the epoch puts any event on a collector's clock
(`nvs_runtime::unix_nanos`). A per-request epoch is declined in that static's own doc comment. A
`query` and an `http` are filed with the duration their span already measured, so their start is the
filing reading less that duration and no second clock read crosses a crate. A door reads the root
span's start before the run and `nvs_server::trace::record` reads its end, so the root's window
contains every child's.

**Only the unbounded half moves to the sink.** A recorded request's `query`, `http` and `spawn` events
stay in the bounded vector the OTLP push reads (`SPAN_EVENT_CEILING`). The `call` events, which grow
with the call count, go to the sink and never to the vector. A run that names no sink and sets `TRACE`
writes the calls nowhere, so no buffer needs a new limit.

**The `Core\Debug` members are one start and stop pair per probe and one read-back:**
`startCoverage(bool $branches = false)`, `stopCoverage()`, `coverage()` returning a
`map<string, map<int, int>>` of path to line to count, `startTrace(string $path)`, `stopTrace()`,
`startProfile(string $path)` and `stopProfile()`. A sink is a file path, admitted only under the
`debug.trace` or `debug.profile` grant (`crates/nvs-config/src/capability.rs:146-149`). A profile from
`Core\Debug` is written in the speedscope format; Callgrind comes from the CLI flag. Why: a pair per
probe needs no mode string (`rule:core-api/no-mode-strings`), and one format per member keeps the surface
small (simplicity). Stage 2's rule may rename a member to match `Core`'s naming and records it there.

**A profile is written to the working folder:** `callgrind.out.<pid>` (the name KCachegrind opens by
default) and `profile.<pid>.speedscope.json`, with `--profile-out <path>` to choose another. Stage 4's
rule item records it.

**View Profile opens an embedded speedscope.** The user decided this. The extension bundles
speedscope's built viewer and opens the file in a webview, so a profile never leaves the machine and it
works in a remote window. `rule:ide/the-extension-builds-no-ui-the-editor-already-has` names the
bundled webview and declines speedscope.app. **The viewer's source is the `speedscope` npm package**
(MIT), a `devDependency` of `editors/vscode/package.json` at a caret range with
`editors/vscode/package-lock.json` pinning the version. The `bundle` script copies its built
`dist/release/` folder and its `LICENSE` into `out/speedscope/`, so `vsce package --no-dependencies`
ships it. The webview opens `index.html#profileURL=<the file's webview URI>`, with `localResourceRoots`
the viewer folder and the profile's folder only. Why: no source build and no blob committed to the tree
(security, simplicity); the lockfile names the exact code that runs.

**Each export is checked by its own format, with no new dependency.** No JSON Schema validator is in
`Cargo.toml` or `package.json`, and none is added. The speedscope test checks what the published
`file-format-schema.json` in the pinned package requires: the `$schema` URL, `shared.frames`, and an
`evented` profile whose open and close events balance with `at` never decreasing. The Callgrind test
checks the format's header (`events:`), each `fn=` block and its `calls=` lines, and that the self costs
sum to the total. Why: a hand-written check of the required keys catches the same breaks without a
validator to audit (security, simplicity).

**Trade-offs, all together.** Request path: one clock read per event, and only while a bit that records
events is on. A request with every bit off pays nothing new. Memory: the call half no longer grows in
memory, so a traced request holds less than today; a profile holds one entry per frame in flight and one
name per distinct function. Usability: a timeline and a call graph that open in existing viewers, and a
stated distortion in call-heavy code. Simplicity: one profiler and two flags. The extension grows by the
bundled viewer.

**Lanes.** Stage 5 touches only `editors/vscode/` and needs a fixture speedscope file, not this goal's
exporter, so it runs as a side lane beside stages 1 to 4. Stage 2 (`crates/nvs-stdlib/src/debug.rs`,
`crates/nvs-config/src/capability.rs`) is disjoint from stage 1 and may run beside it. Stages 1, 6 and 4
share `crates/nvs-runtime/src/ctx/trace.rs` and run in that order. Stage 6 also needs stage 2's members,
stage 7 needs stage 6, stage 3 needs stage 6's sink (`crates/nvs-host/` is its own), and stage 8 needs
stage 4's flags.

## Stages

### 1. every trace event has a time on one clock

**Does:** each `TraceEvent` carries its start and end on the process epoch, and an OTLP span has a
window.

- [x] `TraceEvent` gains a start and an end reading on the epoch (`rule:observability/trace-events-carry-a-kind`). `crates/nvs-runtime/src/ctx/trace.rs:102`
- [x] The `call` probe pair, `gc`, `spawn`, `query` and `http` each fill them where they already read a clock (`rule:observability/gc-pause-is-its-own-event`, `rule:observability/spawn-is-its-own-event`). `crates/nvs-runtime/src/ctx/trace.rs:407`, `crates/nvs-runtime/src/ctx/trace.rs:453`, `crates/nvs-runtime/src/ctx/trace.rs:769`, `crates/nvs-runtime/src/ctx/trace.rs:329`
- [x] The epoch is taken with one wall-clock reading, and each `Span` carries the event's window on that clock (`rule:observability/four-kinds-become-a-span`). `crates/nvs-runtime/src/ctx/trace.rs:27`, `crates/nvs-server/src/trace.rs:182`, `crates/nvs-server/src/trace.rs:217`
- [x] The OTLP encoder writes each span's start and end from them. `crates/nvs-server/src/otlp.rs:983`
- [x] A Rust test that a span's window contains its children's (`rule:observability/four-kinds-become-a-span`). `crates/nvs-server/src/otlp.rs:1324`
- [x] The gap record deleted, and the comment that cites it rewritten. `crates/nvs-server/src/otlp.rs:978`

### 2. `Core\Debug` starts and stops each probe

**Does:** a program turns coverage, trace and profile on and off mid-request and reads coverage back.

- [x] The rule names the members, their parameters and the coverage map's shape, as the standing decision settles them (`rule:testing/debug-surface`, `rule:core-api/no-mode-strings`). `docs/rules/testing/debug-surface.md:1`
- [x] The members join `Core\Debug` with their reference cards, each narrowing within `[debug] mode` (`rule:testing/debug-mode-directive`, `rule:core-api/reference-card`). `crates/nvs-stdlib/src/debug.rs:117`
- [x] A trace or profile path is admitted only under the `debug.trace` or `debug.profile` grant (`rule:testing/debug-probes`). `crates/nvs-config/src/capability.rs:146`
- [x] Rust tests: a start and stop around one call, a coverage read-back, a path refused without its grant, and a request that cannot widen `[debug] mode` (`rule:testing/debug-surface`). `crates/nvs-stdlib/src/debug.rs:409`

### 6. a trace streams to a named sink

**Does:** the `call` events go to a sink as they close, and never into memory.

- [x] A sink writes the `call` events as newline-delimited JSON as they close (`rule:testing/debug-surface`). `crates/nvs-runtime/src/ctx/trace.rs:329`
- [x] The vector keeps only the bounded span events, and `startTrace`'s path opens the sink. `crates/nvs-runtime/src/ctx/mod.rs:1381`, `crates/nvs-stdlib/src/debug.rs:117`
- [x] Rust tests: a traced loop of many calls holds no `call` event in the vector, and the sink file has one line per call (`rule:testing/debug-probes`). `crates/nvs-runtime/src/ctx/trace.rs:1324`
- [x] Every host that installs a statement table locates it with `StmtLines`, and `nvs run` installs one when its ceiling has `coverage`, so `Core\Debug::coverage` answers from source (`rule:testing/debug-surface`). `crates/nvs-cli/src/runner.rs:755`, `crates/nvs-cli/src/main.rs:2163`, `crates/nvs-runtime/src/ctx/trace.rs:832`

### 7. the new `Core\Debug` members have their feature proofs

**Does:** each member stage 2 added owes nothing.

**Split:** the session runs `bun nv proofs --id 'core/Debug'` and pastes the owed paths under the items
below. Writing subagents, one after another: the `about.md` files and three examples per member; then
the Novis tests and the attacks; then the benches. The session checks each batch and ticks it.

- [ ] `about.md` and three examples for each new member (`rule:testing/feature-proofs`). `docs/examples/core/Debug/startTrace/about.md:1`
- [ ] A Novis test and an attack for each: a request that writes a trace or profile without the grant fails (`rule:testing/feature-proofs`). `tests/hostile/core/Debug/startTrace/01-without-the-grant.nvs:1`
- [ ] A bench for each, with its growth (`rule:testing/member-perf-ledger`). `benches/members/core/Debug/startTrace.nvs:1`
- [ ] `bun nv proofs --id 'core/Debug'` prints nothing owed. `tools/nv/proofs/roster.ts:1`

### 3. a spawned isolate's probe data comes back on its result

**Does:** a child starts from its parent's flag word, may only narrow it, and its coverage and trace
come back on its result.

**Gate:** stage 6's sink exists (`crates/nvs-runtime/src/ctx/trace.rs` writes `call` events to a sink).

- [ ] A child inherits the parent's flag word at spawn and can only narrow it, under every spawn form (`rule:testing/debug-surface`). `crates/nvs-host/src/group.rs:561`, `crates/nvs-host/src/placed.rs:132`
- [ ] Its coverage and its sink's events return as fields on the result, by the copy-out a usage figure uses (`rule:security/isolate-shares-nothing`). `crates/nvs-host/src/placed.rs:278`
- [ ] Rust tests over a `spawn worker` child, and that the parent's live state is unchanged by the child's data (`rule:security/isolate-shares-nothing`). `crates/nvs-host/src/placed.rs:469`

### 4. the profiler times every call, and `--profile` writes Callgrind or speedscope

**Does:** `PROFILE` times the call probes, and two flags on `nvs run` and `nvs test` write the two
exports.

- [ ] The rule says where each file goes, as the standing decision says (`rule:observability/speedscope-timeline-export`). `docs/rules/observability/speedscope-timeline-export.md:1`
- [ ] The call probes read `PROFILE` and accumulate self and inclusive time per function (`rule:testing/debug-surface`). `crates/nvs-runtime/src/ctx/trace.rs:769`, `crates/nvs-runtime/src/ctx/trace.rs:803`
- [ ] `--profile=callgrind`, `--profile=speedscope` and `--profile-out` on `nvs run` and `nvs test`, each writing as the run goes, with help text that states the probe's cost. `crates/nvs-cli/src/run.rs:15`, `crates/nvs-cli/src/runner.rs:142`
- [ ] Rust tests over a fixture: the Callgrind file and the speedscope file pass the format checks the standing decision names (`rule:observability/speedscope-timeline-export`). `crates/nvs-cli/src/runner.rs:3458`
- [ ] Both rules `shipped`, their `**Not on disk.**` paragraphs rewritten (`rule:testing/debug-surface`, `rule:observability/speedscope-timeline-export`). `data/rules/testing/debug-surface.json:3`, `data/rules/observability/speedscope-timeline-export.json:3`, `docs/rules/testing/debug-surface.md:28`

### 5. VS Code opens a profile

**Does:** a *View Profile* command opens a speedscope file in a webview. It needs only a fixture
speedscope file, so it runs beside stages 1 to 4.

- [ ] The `speedscope` devDependency, and the `bundle` step that copies its `dist/release/` and licence into `out/speedscope/`. `editors/vscode/package.json:352`
- [ ] The command and its contribution land (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). `editors/vscode/src/extension.ts:129`, `editors/vscode/package.json:202`
- [ ] The webview loads only the bundled viewer and the chosen file, under a content security policy that allows nothing else (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). `editors/vscode/src/profile.ts:1`
- [ ] A headless test opens a fixture speedscope file and checks the policy and the resource roots. `editors/vscode/test/profile.test.ts:1`

### 8. the profiler has its feature proofs

**Does:** the profiler is a documented tool that owes nothing.

**Gate:** stage 4's `--profile` flags exist (`nvs run --help` lists `--profile`).

**Split:** the session adds the chapter section; then one writing subagent, after the bench, writes
`about.md`, the three examples and the attack, under § *Text an end user reads*. Its brief names the paths
`bun nv proofs --id 'tools:cli/profiling'` prints.

- [ ] A `# Profiling` section in the CLI chapter, which states the probe's cost. `docs/reference/tools/10-cli.md:302`
- [ ] A bench of the probe's cost per call, with its growth (`rule:testing/member-perf-ledger`). `benches/members/tools/cli/profiling.nvs:1`
- [ ] `about.md` and three examples (`rule:testing/feature-proofs`). `docs/examples/tools/cli/profiling/about.md:1`
- [ ] An attack: a request cannot write a profile without the grant (`rule:testing/feature-proofs`). `tests/hostile/tools/cli/profiling/01-a-profile-without-the-grant.nvs:1`
- [ ] `bun nv proofs --id 'tools:cli/profiling'` prints nothing owed, and `bun nv owners --closes trace-timeline` passes. `tools/nv/proofs/roster.ts:209`

### 9. the goal is verified once, and every failure is fixed

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

Stages 1, 2 and 6 are done: the seven `Core\Debug` members exist, and a trace and coverage work from
source. The sweep's nightly reds ran on `3ea6f15`, before the commits that fixed each.

- **The trace streams.** `startTrace` creates a `nvs_runtime::TraceSink`
  (`crates/nvs-runtime/src/ctx/trace.rs:783`); `stopTrace` closes it and throws on a failed write. One
  line per call as it returns, `{"kind":"call","callee":…,"status":"ok","start":…,"end":…}` in
  trace-clock nanoseconds. A call still open at the stop has no line.
- **No `call` is in `Ctx::trace`** (`:381`), and `records_spans` holds a `TRACE` run to
  `SPAN_EVENT_CEILING` too (`:294`).
- **Coverage is located.** `coverage::Sites::table` (`crates/nvs-cli/src/coverage.rs:278`) shares one
  `StmtLines` across `nvs test`'s run and per-test tables, and `nvs run` installs a table under a
  `coverage` ceiling (`crates/nvs-cli/src/main.rs:2093`). Both checked by a scratch run.

The rewritten trace tests passed once by name (`ctx::trace::tests::`, codegen `calls` and `probes`,
`debug::tests::a_trace_*`). Expected red at the gate: `bun nv proofs --gate` for the seven members
(stage 7's), and any `nvs meta --json` or reference-card snapshot listing `Core\Debug`'s members.

### Next

**Stage 7: the new `Core\Debug` members have their feature proofs.** Follow its `**Split:**` line. Run
`bun nv proofs --id 'core/Debug'` first, and paste the owed paths under the stage's items. Then send out
writing subagents one after another, each with a brief that names its ceiling. The first writes the
`about.md` files and three examples per member. The second writes the Novis tests and attacks, and the
third the benches. An example or test that traces needs `[debug] mode` and a `[capabilities.debug]
trace` grant in its configuration, the way `crates/nvs-stdlib/src/debug.rs:1060` writes them. Every
comment follows AGENTS.md § *Text an end user reads*. Stage 5 stays a side lane (`editors/vscode/`), and
stage 3 (`crates/nvs-host/`) can follow.

- [ ] `about.md` and three examples for each new member. `docs/examples/core/Debug/startTrace/about.md:1`
- [ ] A Novis test and an attack for each: a request that writes a trace or profile without the grant fails. `tests/hostile/core/Debug/startTrace/01-without-the-grant.nvs:1`
- [ ] A bench for each, with its growth. `benches/members/core/Debug/startTrace.nvs:1`
- [ ] `bun nv proofs --id 'core/Debug'` prints nothing owed. `tools/nv/proofs/roster.ts:1`

### Backlog

- `rule:ide/the-extension-builds-no-ui-the-editor-already-has` also covers the debugger wiring, which is `debugger-adapter`'s (`docs/rules/ide/the-extension-builds-no-ui-the-editor-already-has.md`).
- `debug-viewer` hands a profile to speedscope in this goal's format (`docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md`).
- From `mutation-testing`: a threshold flag that fails the run below a mutation score (`docs/rules/testing/mutation-testing.md`).
- From `mutation-testing`: more operators, such as a removed call statement, added in `crates/nvs-ir/src/lower/mutate.rs`.
- From `mutation-testing`: a test past its `[limits] cpu_time` reports the runtime's line, which names a request. A line naming the test and its limit would match `Outcome::past_wall_time`. `crates/nvs-cli/src/runner.rs:477`
- From `mutation-testing`: a property's counterexample prints an enum case as its number, such as `new Item("", 1, 0)`. The case name would read better. `crates/nvs-cli/src/runner.rs:2357`
- From `mutation-testing`: a long failing string or list spends the 1000-call shrink budget on one-item removals, so its counterexample stays long. Removing chunks first would shrink further. `crates/nvs-types/src/property/shrink.rs:362`
- From `mutation-testing`: a workspace folder with programs and no `.nvst` case pays one compile of the folder when the editor lists its cases, and the client reads none of those records. `editors/vscode/src/tests.ts:143`
- From `mutation-testing`: a case label mixes separators on Windows, such as `tests/conformance/core\x.nvst`, in a report and a listing. `crates/nvs-test/src/lib.rs:225`
- From `mutation-testing`: the "it asserted nothing" report line names a rule id, and text a user reads names none. `crates/nvs-cli/src/runner.rs:3375`
- From `mutation-testing`: `bun nv scaling --budget` names 45 benches over their stored share, among them `core/Metrics/*`, `novis/Spreadsheet-*` and `the-pipeline-operator`, none of them a `Core\Test` bench. `data/perf/bench-budget.json:1`
