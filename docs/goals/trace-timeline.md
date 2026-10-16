---
looper: 1
title: "Trace events have times, and the profiler exports a timeline and a call graph"
summary: "Every trace event has a time on one clock, Core\\Debug streams a trace and a profile to a named sink, nvs run and nvs test write --profile=callgrind and --profile=speedscope, VS Code opens the speedscope file, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-stdlib/src/debug.rs, crates/nvs-cli/src/main.rs]
stages:
  - id: 1
    title: every trace event has a time on one clock
    status: open
    context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-server/src/trace.rs, crates/nvs-server/src/otlp.rs, data/gaps/nvs-server/span-has-no-window.json, docs/rules/observability/four-kinds-become-a-span.md]
    extra:
      context:
        rules: [observability/trace-events-carry-a-kind, observability/four-kinds-become-a-span, observability/gc-pause-is-its-own-event, observability/spawn-is-its-own-event]
  - id: 2
    title: "`Core\\Debug` starts and stops each probe, and a trace streams to a named sink"
    status: open
    context: [crates/nvs-stdlib/src/debug.rs, crates/nvs-runtime/src/ctx/mod.rs, crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-config/src/capability.rs, docs/rules/testing/debug-surface.md]
    extra:
      context:
        rules: [testing/debug-surface, testing/debug-probes, testing/debug-mode-directive, security/secret-qualifier]
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
    context: [crates/nvs-runtime/src/ctx/trace.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/runner.rs, docs/rules/observability/speedscope-timeline-export.md, docs/rules/testing/debug-surface.md]
    extra:
      context:
        rules: [observability/speedscope-timeline-export, testing/debug-surface, testing/debug-probes, testing/report-formats]
  - id: 5
    title: VS Code opens a profile, and the profiler has its feature proofs
    status: open
    context: [editors/vscode/src/extension.ts, editors/vscode/package.json, docs/rules/ide/the-extension-builds-no-ui-the-editor-already-has.md, docs/reference/tools/10-cli.md]
    extra:
      context:
        rules: [ide/the-extension-builds-no-ui-the-editor-already-has, observability/speedscope-timeline-export, testing/feature-proofs]
checks:
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, trace-timeline]
    reach: true
    floor: false
extra:
  milestone: M10
---

# Trace events have times, and the profiler exports a timeline and a call graph

## Why

A trace event records what happened and not when. `TraceEvent` has a kind, a label and a status
(`crates/nvs-runtime/src/ctx/trace.rs:102-114`), so an OTLP span has no duration
(`data/gaps/nvs-server/span-has-no-window.json`) and no timeline can be drawn. The call half of the
trace grows in a vector without bound (`crates/nvs-runtime/src/ctx/mod.rs:1350-1369`), which is why
`TRACE` is off for every served request. The `PROFILE` bit exists and no probe reads it
(`crates/nvs-runtime/src/ctx/trace.rs:744-794`). `Core\Debug` can dump and render a value and cannot
start or stop a probe (`crates/nvs-stdlib/src/debug.rs:120-139`). This goal gives every event a time,
streams the unbounded half to a sink, and builds the one profiler with its two exports.
`debug-viewer` reads the timeline this goal writes.

## Standing decisions

**One profiler, two exports.** The user decided this. Only the probe-based profiler is built:
`nvs run --profile=callgrind` and `--profile=speedscope`, and the same under `nvs test`. There is no
sampling profiler. Its docs say that the probe per call makes call-heavy code look slower. VS Code's
*View Profile* command opens the speedscope file. `rule:testing/debug-surface` and
`rule:observability/speedscope-timeline-export` hold it.

**One clock for every event.** The planning session's call. Every event carries its time as a reading of
the one process epoch `SPAWN_EPOCH` already is (`crates/nvs-runtime/src/ctx/trace.rs:27`). One wall-clock
reading taken with the epoch puts any event on a collector's clock. A per-request epoch is declined in
that static's own doc comment.

**Only the unbounded half moves to the sink.** The planning session's call. A recorded request's
`query`, `http` and `spawn` events stay in the bounded vector the OTLP push reads
(`SPAN_EVENT_CEILING`). The `call` events, which grow with the call count, go to the sink and never to
the vector. A run that names no sink and sets `TRACE` writes the calls nowhere, so no buffer needs a
new limit.

**Where a profile is written is settled in the rule in stage 4.** The planning session's call is the
working folder, `callgrind.out.<pid>` (the name KCachegrind opens by default) and
`profile.<pid>.speedscope.json`, with `--profile-out <path>` to choose another.

**View Profile opens an embedded speedscope.** The planning session's call. The extension bundles
speedscope's built viewer and opens the file in a webview, so a profile never leaves the machine and it
works in a remote window. `rule:ide/the-extension-builds-no-ui-the-editor-already-has` admits a webview
or speedscope.app. Stage 5 writes the choice into that rule.

**Trade-offs, all together.** Request path: one clock read per event, and only while a bit that records
events is on. A request with every bit off pays nothing new. Memory: the call half no longer grows in
memory, so a traced request holds less than today; a profile holds one entry per frame in flight and one
name per distinct function. Usability: a timeline and a call graph that open in existing viewers, and a
stated distortion in call-heavy code. Simplicity: one profiler and two flags. The extension grows by the
bundled viewer.

## Stages

### 1. every trace event has a time on one clock

**Does:** each `TraceEvent` carries its start and end on the process epoch, and an OTLP span has a
window.

- [ ] `TraceEvent` gains a start and an end reading on the epoch, and the `call` probe pair, `gc`, `spawn`, `query` and `http` each fill them where they already read a clock (`rule:observability/trace-events-carry-a-kind`). `crates/nvs-runtime/src/ctx/trace.rs:102-114`, `crates/nvs-runtime/src/ctx/trace.rs:180-195`, `crates/nvs-runtime/src/ctx/trace.rs:440-448`
- [ ] The epoch is taken with one wall-clock reading, and the OTLP encoder writes each span's start and end from them (`rule:observability/four-kinds-become-a-span`). `crates/nvs-server/src/trace.rs:182`, `crates/nvs-server/src/otlp.rs:184`, `crates/nvs-server/src/otlp.rs:980`
- [ ] A test that a span's window contains its children's, and `data/gaps/nvs-server/span-has-no-window.json` deleted.

### 2. `Core\Debug` starts and stops each probe, and a trace streams to a named sink

**Does:** a program turns coverage, trace and profile on and off mid-request, reads coverage back, and
a trace goes to a sink rather than into memory.

- [ ] The rule names the members, their parameters and the coverage map's shape before the code (`rule:testing/debug-surface`, `rule:core-api/no-mode-strings`). `docs/rules/testing/debug-surface.md`
- [ ] The members join `Core\Debug`, each narrowing within `[debug] mode` and writing only under the `debug.trace` or `debug.profile` grant (`rule:testing/debug-mode-directive`). `crates/nvs-stdlib/src/debug.rs:120-139`, `crates/nvs-config/src/capability.rs:394-395`
- [ ] A sink writes the `call` events as newline-delimited JSON as they close, and the vector keeps only the bounded span events. `crates/nvs-runtime/src/ctx/mod.rs:1350-1369`, `crates/nvs-runtime/src/ctx/trace.rs:305`
- [ ] Tests from Novis and from Rust: a start and stop around one call, a coverage read-back, a sink refused without its grant, and a request that cannot widen `[debug] mode`. Examples and reference cards for each member.

### 3. a spawned isolate's probe data comes back on its result

**Does:** a child starts from its parent's flag word, may only narrow it, and its coverage and trace
come back on its result.

- [ ] A child inherits the parent's flag word at spawn and can only narrow it, under every spawn form. `crates/nvs-host/src/group.rs:552`, `crates/nvs-host/src/placed.rs:1-8`
- [ ] Its coverage and its sink's events return as fields on the result, by the copy-out a usage figure uses (`rule:security/isolate-shares-nothing`).
- [ ] A test over a `spawn worker` child, and one that the parent's live state is unchanged by the child's data.

### 4. the profiler times every call, and `--profile` writes Callgrind or speedscope

**Does:** `PROFILE` times the call probes, and two flags on `nvs run` and `nvs test` write the two
exports.

- [ ] The rule says where each file goes, as the standing decision says (`rule:observability/speedscope-timeline-export`). `docs/rules/observability/speedscope-timeline-export.md`
- [ ] The call probes read `PROFILE` and accumulate self and inclusive time per function. `crates/nvs-runtime/src/ctx/trace.rs:744-794`
- [ ] `--profile=callgrind` and `--profile=speedscope` on `nvs run` and `nvs test`, each writing as the run goes. `crates/nvs-cli/src/main.rs:421`, `crates/nvs-cli/src/main.rs:574`
- [ ] A Callgrind file KCachegrind's format accepts and a speedscope file that matches speedscope's published schema, each checked by a test over a fixture. The help text states the probe's cost. Both rules `shipped`.

### 5. VS Code opens a profile, and the profiler has its feature proofs

**Does:** a *View Profile* command opens a speedscope file in a webview, and the profiler is a
documented tool.

- [ ] The rule names the bundled webview, and the command and its contribution land (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). `editors/vscode/src/extension.ts:129-153`, `editors/vscode/package.json:204`
- [ ] The webview loads only the bundled viewer and the chosen file, under a content security policy that allows nothing else.
- [ ] A section for the profiler in `docs/reference/tools/10-cli.md`, and `bun nv proofs --id` on its id passes: `about.md`, examples, a bench of the probe's cost per call, and an attack that a request cannot write a profile without the grant.
- [ ] `bun nv owners --closes trace-timeline` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written. `crates/nvs-runtime/src/ctx/mod.rs` was being edited by another session at the time, so its
line numbers may have moved: find the `trace` field by name.

### Next

Stage 1, a time on every event, on `crates/nvs-runtime/src/ctx/trace.rs` and
`crates/nvs-server/src/otlp.rs`:
- the start and end on `TraceEvent` (`rule:observability/trace-events-carry-a-kind`). `crates/nvs-runtime/src/ctx/trace.rs:102`
- each kind fills them where it reads a clock. `crates/nvs-runtime/src/ctx/trace.rs:440`
- the OTLP span's window (`rule:observability/four-kinds-become-a-span`). `crates/nvs-server/src/otlp.rs:980`

### Backlog

- `rule:ide/the-extension-builds-no-ui-the-editor-already-has` also covers the debugger wiring, which is `debugger-adapter`'s (`docs/rules/ide/the-extension-builds-no-ui-the-editor-already-has.md`).
- `debug-viewer` hands a profile to speedscope in this goal's format (`docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md`).
