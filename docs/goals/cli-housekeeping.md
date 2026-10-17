---
looper: 1
title: The cache commands, the update check and telemetry exist
summary: "nvs cache gc and clear, nvs update-check and nvs telemetry are built as their rules say, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-cli/src/main.rs, crates/nvs-config/src/data.rs]
stages:
  - id: 1
    title: "`nvs cache gc` and `nvs cache clear`"
    status: open
    context: [crates/nvs-cli/src/cache.rs, crates/nvs-cli/src/main.rs, docs/rules/packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape.md, docs/rules/packaging/eviction-rides-the-cold-miss-at-a-probability.md]
    extra:
      context:
        rules: [packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape, packaging/eviction-rides-the-cold-miss-at-a-probability]
  - id: 2
    title: "the endpoints are configuration, and `nvs update-check` reads GitHub's latest release"
    status: open
    context: [crates/nvs-cli/src/main.rs, docs/rules/config/telemetry-and-update-endpoints-are-configuration.md, docs/rules/tooling/update-check-states-the-whole-picture.md]
    extra:
      context:
        rules: [config/telemetry-and-update-endpoints-are-configuration, tooling/update-check-states-the-whole-picture, tooling/telemetry-is-two-opt-ins]
  - id: 3
    title: "`nvs telemetry` records a closed set of counters locally"
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-config/src/data.rs, docs/rules/tooling/telemetry-is-two-opt-ins.md, docs/rules/tooling/telemetry-counters-are-a-closed-set.md, docs/rules/tooling/telemetry-show-prints-the-payload.md]
    extra:
      context:
        rules: [tooling/telemetry-is-two-opt-ins, tooling/telemetry-counters-are-a-closed-set, tooling/telemetry-show-prints-the-payload]
  - id: 4
    title: "an upload is carried, never fails the command, and never runs in a serving process"
    status: open
    context: [crates/nvs-cli/src/main.rs, docs/rules/tooling/a-serving-process-never-uploads.md, docs/rules/tooling/a-failed-upload-never-fails-the-command.md]
    extra:
      context:
        rules: [tooling/a-serving-process-never-uploads, tooling/a-failed-upload-never-fails-the-command, config/telemetry-and-update-endpoints-are-configuration]
checks:
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, cli-housekeeping]
    reach: true
    floor: false
---

# The cache commands, the update check and telemetry exist

## Why

Eight designed rules describe three small commands that do not exist. The artifact cache has its
deterministic walk, `Cache::gc` (`crates/nvs-cli/src/cache.rs:1435`), but no command calls it, so an
operator cannot empty or trim the cache on demand. `nvs update-check` and `nvs telemetry` are fully
designed in the tooling chapter, and nothing in `crates/` implements them. This goal builds all
three. They touch no request path, and each one is tested against a local listener or a temporary
directory.

## Standing decisions

**The update check reads GitHub's Releases API, and telemetry ships with no default upload URL.**
The user decided this. `nvs telemetry on` records locally and says that nothing is uploaded until a
collector exists. The upload code is built and tested in full against a local listener through
`NVS_TELEMETRY_URL`. A collector is not this goal's work.
`rule:config/telemetry-and-update-endpoints-are-configuration` holds it.

**Open points are settled in the rule before the code.** The planning session's call. Stage 1
settles the cache commands' output, `.tmp-` files, an entry Windows keeps open, and a relocated
cache. Stage 3 settles where the consent and counter files live, and whether `DO_NOT_TRACK` and a
CI environment turn telemetry off. Stage 4 settles how a serving process is detected, for
`nvs serve`, `nvs lsp`, `nvs dap` and `nvs debug`. A point with no safe default goes to the user.

**Trade-offs.** No program changes, and nothing runs on the request path. `nvs serve` pays one
buffered counter append at startup when telemetry is on, before the listener binds
(`rule:tooling/telemetry-counters-are-a-closed-set`). The counter file is a few KiB per user. The
update check is one HTTPS request per run, only when the user runs it. Three new subcommands add to
the CLI surface, and each is one verb a user already expects.

## Stages

### 1. `nvs cache gc` and `nvs cache clear`

**Does:** two subcommands over the existing cache, as
`rule:packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape` says.

- [ ] The rule settles the output (bytes freed, and a `--json` form), that a `.tmp-` file is left alone, that an entry Windows refuses to delete is skipped and counted, and that both commands act on the directory `file_cache_dir` names when it is set. `crates/nvs-cli/src/cache.rs:1427`, `crates/nvs-cli/src/cache.rs:1465`, `crates/nvs-cli/src/cache.rs:1647`
- [ ] `nvs cache gc` calls `Cache::gc` without the probability roll. `crates/nvs-cli/src/cache.rs:1435`, `crates/nvs-cli/src/cache.rs:1403`
- [ ] `nvs cache clear` deletes every `.nvsc` entry in the store, the same way. `crates/nvs-cli/src/cache.rs:1744`
- [ ] Tests over a temporary cache, the help text in the binary, and the rule `shipped`.

### 2. the endpoints are configuration, and `nvs update-check` reads GitHub's latest release

**Does:** `NVS_UPDATE_URL` and `NVS_TELEMETRY_URL` are read, and `nvs update-check` prints the
whole picture its rule names.

- [ ] Both variables are read, with GitHub's latest-release endpoint as the update default and no telemetry default (`rule:config/telemetry-and-update-endpoints-are-configuration`). `crates/nvs-cli/src/main.rs:360`
- [ ] `nvs update-check` streams one line per step, reports the telemetry switch, the local version and the latest version, and has a `--json` form with a schema version (`rule:tooling/update-check-states-the-whole-picture`).
- [ ] It exits `0` whether or not an update exists and whether or not the request succeeded.
- [ ] Tests against a local listener that serves a Releases API document, and both rules `shipped`.

### 3. `nvs telemetry` records a closed set of counters locally

**Does:** `nvs telemetry on|off|status|reset|show` keeps consent and counters in plain files, and
every invocation increments one counter from the compiled-in command table.

- [ ] The rule settles where the files live in the data folder, and whether `DO_NOT_TRACK` and a CI environment turn telemetry off (`rule:tooling/telemetry-is-two-opt-ins`). `crates/nvs-config/src/data.rs:459`
- [ ] The five verbs, and `on` says that nothing is uploaded while no upload URL is set.
- [ ] A counter is `invocation.<subcommand>` from the command table, or `invocation.unknown`, and the argv never becomes a name (`rule:tooling/telemetry-counters-are-a-closed-set`). `crates/nvs-cli/src/main.rs:360`
- [ ] `nvs telemetry show` prints the exact payload, and a test compares it with what a local listener receives (`rule:tooling/telemetry-show-prints-the-payload`).
- [ ] The three rules `shipped`.

### 4. an upload is carried, never fails the command, and never runs in a serving process

**Does:** the previous finished week's counters go out once, carried by a short-lived command, in
at most three seconds.

- [ ] The rule settles how a process counts as serving: `nvs serve`, `nvs lsp`, `nvs dap` and `nvs debug` record and never upload (`rule:tooling/a-serving-process-never-uploads`).
- [ ] The weekly gate, the randomised point in the window, and `nvs telemetry upload`. `nvs update-check` carries an upload when telemetry is on.
- [ ] A three-second budget, one honest line on failure, and the command's own exit status untouched (`rule:tooling/a-failed-upload-never-fails-the-command`).
- [ ] Tests against a local listener, among them `nvs serve` under load making no outbound connection. Both rules `shipped`, and `bun nv owners --closes cli-housekeeping` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written.

### Next

Stage 1, the cache commands, on `crates/nvs-cli/src/cache.rs` and `crates/nvs-cli/src/main.rs`:
- settle the output, `.tmp-` files, Windows and the relocated cache in the rule (`rule:packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape`). `crates/nvs-cli/src/cache.rs:1427`
- `nvs cache gc` over `Cache::gc`. `crates/nvs-cli/src/cache.rs:1435`
- `nvs cache clear`. `crates/nvs-cli/src/cache.rs:1744`

### Backlog

- A telemetry collector and its default upload URL are not this goal's work (`docs/rules/config/telemetry-and-update-endpoints-are-configuration.md`).
- `nvs dap` and `nvs debug` do not exist yet. Stage 4's serving check names them so the goals that build them inherit it (`docs/goals/debugger-adapter.md`, `docs/goals/debug-service.md`).
