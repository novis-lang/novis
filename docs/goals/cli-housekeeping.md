---
looper: 1
title: The cache commands, the update check and telemetry exist
summary: "nvs cache gc and clear, nvs update-check and nvs telemetry are built as their rules say, every project walk skips what .gitignore and .ignore ignore, every designed rule and gap record this goal owns is closed, and the checks below pass."
context: [crates/nvs-cli/src/main.rs, crates/nvs-config/src/data.rs]
stages:
  - id: 1
    title: "`nvs cache gc` and `nvs cache clear`"
    status: done
    context: [crates/nvs-cli/src/cache.rs, crates/nvs-cli/src/cache_cmd.rs, docs/rules/packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape.md, docs/rules/packaging/eviction-rides-the-cold-miss-at-a-probability.md]
    extra:
      context:
        rules: [packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape, packaging/eviction-rides-the-cold-miss-at-a-probability]
  - id: 2
    title: "the endpoints are configuration, and `nvs update-check` reads GitHub's latest release"
    status: done
    context: [crates/nvs-cli/src/endpoints.rs, crates/nvs-cli/src/update_check.rs, crates/nvs-stdlib/src/http/transport.rs, docs/rules/config/telemetry-and-update-endpoints-are-configuration.md, docs/rules/tooling/update-check-states-the-whole-picture.md]
    extra:
      context:
        rules: [config/telemetry-and-update-endpoints-are-configuration, tooling/update-check-states-the-whole-picture, tooling/telemetry-is-two-opt-ins]
  - id: 3
    title: "`nvs telemetry` records a closed set of counters locally"
    status: open
    context: [crates/nvs-cli/src/telemetry.rs, crates/nvs-cli/src/endpoints.rs, docs/rules/tooling/telemetry-is-two-opt-ins.md, docs/rules/tooling/telemetry-counters-are-a-closed-set.md, docs/rules/tooling/telemetry-show-prints-the-payload.md]
    extra:
      context:
        rules: [tooling/telemetry-is-two-opt-ins, tooling/telemetry-counters-are-a-closed-set, tooling/telemetry-show-prints-the-payload]
  - id: 4
    title: "an upload is carried, never fails the command, and never runs in a serving process"
    status: open
    context: [crates/nvs-cli/src/telemetry.rs, crates/nvs-cli/src/update_check.rs, docs/rules/tooling/a-serving-process-never-uploads.md, docs/rules/tooling/a-failed-upload-never-fails-the-command.md]
    extra:
      context:
        rules: [tooling/a-serving-process-never-uploads, tooling/a-failed-upload-never-fails-the-command, config/telemetry-and-update-endpoints-are-configuration]
  - id: 5
    title: "the project walk skips what `.gitignore` ignores, `nvs check` reads a directory, and a `require` that cannot be followed is an error"
    status: open
    context: [crates/nvs-hir/src/lenders.rs, crates/nvs-cli/src/fmt.rs, crates/nvs-lsp/src/index.rs, crates/nvs-hir/src/requires.rs, docs/rules/tooling/nvs-code-answers-about-the-project.md, docs/rules/tooling/fmt-check-writes-nothing.md, docs/rules/statements/require-is-the-only-inclusion-construct.md]
    extra:
      context:
        rules: [tooling/nvs-code-answers-about-the-project, tooling/fmt-check-writes-nothing, testing/a-directory-of-programs-is-one-test-program, statements/require-is-the-only-inclusion-construct, programs/relative-paths-resolve-from-their-file]
  - id: 6
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: cache-help
    stage: 1
    name: nvs cache has gc and clear
    run: [cargo, run, -q, --bin, nvs, --, cache, --help]
    stdout_contains: [gc, clear]
  - id: update-check-help
    stage: 2
    name: nvs update-check exists
    run: [cargo, run, -q, --bin, nvs, --, update-check, --help]
  - id: telemetry-help
    stage: 3
    name: nvs telemetry has its five verbs
    run: [cargo, run, -q, --bin, nvs, --, telemetry, --help]
    stdout_contains: ["on", "off", status, reset, show]
  - id: owners-closes
    stage: 5
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

Stage 5 is three smaller fixes to how the CLI finds files. The project walk reads every `.nvs` file
under the project, including generated and copied trees that `.gitignore` already names, so `nvs code`,
the editor and `nvs check`'s lender search analyse files nobody wrote. `nvs check <directory>` fails
instead of checking the directory. A `require` whose path is not statically known reports nothing, and
every `use` after it fails with `E0306` instead.

## Standing decisions

**The update check reads GitHub's Releases API, and telemetry ships with no default upload URL.**
The user decided this. `nvs telemetry on` records locally and says that nothing is uploaded until a
collector exists. The upload code is built and tested in full against a local listener through
`NVS_TELEMETRY_URL`. A collector is not this goal's work.
`rule:config/telemetry-and-update-endpoints-are-configuration` holds it.

**Each command is its own module, and only the subcommand table is shared.** `nvs cache` is
`crates/nvs-cli/src/cache_cmd.rs` over `Cache` in `cache.rs`, `nvs update-check` is
`update_check.rs`, `nvs telemetry` is `telemetry.rs`, and the two URL variables are read in
`endpoints.rs`. Each adds one variant to `enum Command` (`crates/nvs-cli/src/main.rs:373`) and one
arm to the dispatch (`crates/nvs-cli/src/main.rs:566`). Why: simplicity, and the stages stop
colliding in `main.rs`.

**Outbound HTTPS uses the client the tree already has, and no dependency is added.** The update
check and the upload go through `Core\Http\Client`'s transport (`crates/nvs-stdlib/src/http/transport.rs`,
`rustls`). Why: security; one TLS stack to audit, and `deny.toml` stays as it is.

**The open points are settled, and each stage writes its choice into its rule before the code.**

- *Cache output.* One line, `removed <n> entries, freed <bytes> bytes, skipped <k>`, and `--json`
  prints `{"schema":1,"removed":n,"freed_bytes":b,"skipped":k}`. Exit `0`, also when nothing was
  removed.
- *`.tmp-` files.* Left alone: each belongs to a writer that may be running
  (`crates/nvs-cli/src/cache.rs:1427`). Why: security; deleting another process's half-written
  entry is a race.
- *An entry Windows keeps open.* Skipped and counted in `skipped`, and the command still exits `0`.
  Why: the next `gc` takes it, and failing would make cron noisy for nothing.
- *A relocated cache.* Both commands act on the directory `from_config` resolves
  (`crates/nvs-cli/src/cache.rs:1642`), the one `nvs run` reads. A refused `file_cache_dir` prints
  the same warning as a run and the command acts on the fallback. Why: the command must reach the
  store a run uses, and the trust check stays in one place.
- *Telemetry files.* `<data folder>/telemetry/consent` and `<data folder>/telemetry/counters`,
  created through `data::prepare` (`crates/nvs-config/src/data.rs:459`). `reset` deletes the
  `telemetry/` directory. Why: one directory to inspect or delete, with the data folder's private
  permissions.
- *`DO_NOT_TRACK` and CI.* A non-empty `DO_NOT_TRACK` or `CI` turns telemetry off for that process:
  nothing is recorded and nothing is sent, and `status` names the variable. The update check still
  runs when invoked. Why: security first; an operator's standing opt-out wins over a stored consent.
- *The three payload fields beyond the counters.* `days_active` comes from seven day bits for the
  current week in the counter file. `first` is `true` while the lifetime count of accepted uploads,
  which `update-check` already reports, is zero. `internal-error.<subcommand>` is written by a panic
  hook that `main` installs once the subcommand is known, and the hook appends and returns, so the
  panic itself is unchanged. Why: no new file and no new process, and the hook cannot hide a crash.
- *A serving process.* Each `Command` variant states whether it may carry an upload, and only
  `update-check` and `telemetry upload` say yes. `serve`, `lsp`, `dap`, `debug` and every new command
  record and never upload. Why: an allow-list fails closed when a command is added.
- *The serving test.* `nvs serve` runs with telemetry on, a finished unsent week in the counter
  file and `NVS_TELEMETRY_URL` at a local listener. A loop of requests drives it, and the test
  asserts that the listener accepted zero connections and `invocation.serve` counted once.

**The project walk skips what `.gitignore` and `.ignore` ignore, and nothing is configured.** The one
source walk, `nvs_hir::lenders::sources` (`crates/nvs-hir/src/lenders.rs:239`), serves `nvs code`, the
editor's workspace index (`crates/nvs-lsp/src/index.rs:894`) and `nvs check`'s lender search. It goes
through the `ignore` crate's `WalkBuilder`, the walker ripgrep uses, added to the workspace
`Cargo.toml` and to `nvs-hir`. The settings:

- `.gitignore` and `.ignore` are read in every directory, also outside a git repository.
- The global git excludes file and `.git/info/exclude` are not read. Why: the same tree gives the same
  file list on every machine and in CI.
- `target/`, `vendor/` and a dot-directory are still skipped, and a dot-file is still read, as today.
- A path named on the command line is always taken, even when an ignore file names it.

Why: a project already lists what it does not write in `.gitignore`. **Declined:** an exclude key in
`nvs.toml`, because every project would repeat its `.gitignore` there. The rule that says which files
are analysed (`rule:tooling/nvs-code-answers-about-the-project`, `docs/rules/tooling.md:1894`, "Every
`.nvs` file under it is analysed") names the ignore files, with that Declined entry.

**`nvs fmt` walks with the same function.** `formatter-complete` stage 1 makes the `nvs fmt` walk
(`crates/nvs-cli/src/fmt.rs:177`) skip `vendor/`. Stage 5 replaces that walk with
`nvs_hir::lenders::sources`, so the `vendor/` skip lives in one place, and
`rule:tooling/fmt-check-writes-nothing` says that `nvs fmt` walks the project's files. Gate:
`docs/goals/formatter-complete.md` no longer exists, which holds because it sits ahead of this goal on
the chain.

**`nvs check <directory>` checks the program `nvs test` runs for it.** It reads the directory through
`Program::Directory` (`crates/nvs-cli/src/main.rs:2585`), so the two commands agree
(`rule:testing/a-directory-of-programs-is-one-test-program`).

**A `require` whose path is not statically known is a compile error.** Such a site loads no file while
compiling and always throws when it is reached (`crates/nvs-hir/src/requires.rs:8`), so it can never
work. It gets one new code beside `E0311` (`crates/nvs-diagnostics/src/code/names.rs:64`). Its help
names the literal form, `require './autoload.nvs';`, which resolves from the file it is written in.
`rule:statements/require-is-the-only-inclusion-construct` lists the forms that are statically known:
literal text, a class constant, and `.` between them (`crates/nvs-hir/src/requires.rs:88`).

**Lanes.** Stage 1 (`cache_cmd.rs`, `cache.rs`, and `agent.rs` for the gap) is a lane of its own and can run in any order or
as a side goal. Stages 2 and 3 share `endpoints.rs`, so 3 runs after 2. Stage 4 shares
`telemetry.rs` with 3 and `update_check.rs` with 2, so it runs last. All four share only the
subcommand table in `main.rs`, which is a one-line merge each. Stage 5 (`lenders.rs`, `fmt.rs`,
`requires.rs` and `nvs check`'s entry in `main.rs`) shares no other file with stages 1 to 4 and is a
lane of its own. The whole goal touches no request path. Stages 1 to 4 can run beside the chain as a
side goal. Stage 5 changes `nvs-hir` and waits for its gate, so it does not run while
`compiler-holes` or `formatter-complete` is open.

**Trade-offs.** No program changes, and nothing runs on the request path. `nvs serve` pays one
buffered counter append at startup when telemetry is on, before the listener binds
(`rule:tooling/telemetry-counters-are-a-closed-set`). The counter file is a few KiB per user. The
update check is one HTTPS request per run, only when the user runs it. Three new subcommands add to
the CLI surface, and each is one verb a user already expects. Stage 5 adds the `ignore` crate, the
walker ripgrep uses, with its small set of dependencies. The walk reads each ignore file once per
walk, which costs a few KiB for the parsed patterns while the walk runs, and never runs on the request
path. A dynamic `require` that compiled before and always threw now fails to compile.

## Stages

### 1. `nvs cache gc` and `nvs cache clear`

**Does:** two subcommands over the existing cache, as
`rule:packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape` says.

- [x] The rule states the output, `.tmp-` files, the skipped Windows entry and the relocated cache as § *Standing decisions* settles them (`rule:packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape`). `docs/rules/packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape.md:1`
- [x] `nvs cache gc` calls `Cache::gc` without the probability roll of `maybe_gc`. `crates/nvs-cli/src/cache_cmd.rs:1`, `crates/nvs-cli/src/cache.rs:1435`, `crates/nvs-cli/src/cache.rs:1403`
- [x] `nvs cache clear` deletes every `.nvsc` entry in the store, the same way. `crates/nvs-cli/src/cache_cmd.rs:1`, `crates/nvs-cli/src/cache.rs:1435`
- [x] The `Cache` variant and its dispatch arm. `crates/nvs-cli/src/main.rs:373`, `crates/nvs-cli/src/main.rs:566`
- [x] Tests over a temporary cache: over the cap, under it, a `.tmp-` file kept, `--json`. `crates/nvs-cli/src/cache_cmd.rs:1`
- [x] The help text in the binary. `crates/nvs-cli/src/cache_cmd.rs:1`
- [x] The rule `shipped` (`rule:packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape`). `data/rules/packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape.json:3`

`nvs agent find`, the one gap record this goal owns:

- [x] `find` also matches a section's code spans, with a case for a form no heading names, beside the reachability test. `crates/nvs-cli/src/agent.rs:986`, `crates/nvs-cli/tests/agent.rs:907`
- [x] The gap record deleted. `data/gaps/nvs-cli/agent-find-matches-only-heading-slugs.json:1`

### 2. the endpoints are configuration, and `nvs update-check` reads GitHub's latest release

**Does:** `NVS_UPDATE_URL` and `NVS_TELEMETRY_URL` are read, and `nvs update-check` prints the
whole picture its rule names.

- [x] Both variables are read, with GitHub's latest-release endpoint as the update default and no telemetry default (`rule:config/telemetry-and-update-endpoints-are-configuration`). `crates/nvs-cli/src/endpoints.rs:1`
- [x] `nvs update-check` streams one line per step, reports the telemetry switch, the local version and the latest version, and has a `--json` form with a schema version (`rule:tooling/update-check-states-the-whole-picture`). `crates/nvs-cli/src/update_check.rs:1`
- [x] It exits `0` whether or not an update exists and whether or not the request succeeded. `crates/nvs-cli/src/update_check.rs:1`
- [x] The `UpdateCheck` variant and its dispatch arm. `crates/nvs-cli/src/main.rs:373`, `crates/nvs-cli/src/main.rs:566`
- [x] Tests against a local listener that serves a Releases API document: newer, same, unreachable. `crates/nvs-cli/src/update_check.rs:1`
- [x] Both rules `shipped` (`rule:config/telemetry-and-update-endpoints-are-configuration`, `rule:tooling/update-check-states-the-whole-picture`). `data/rules/config/telemetry-and-update-endpoints-are-configuration.json:3`, `data/rules/tooling/update-check-states-the-whole-picture.json:3`

### 3. `nvs telemetry` records a closed set of counters locally

**Does:** `nvs telemetry on|off|status|reset|show` keeps consent and counters in plain files, and
every invocation increments one counter from the compiled-in command table.

- [ ] The rule states the file paths and the `DO_NOT_TRACK` and CI switch as § *Standing decisions* settles them (`rule:tooling/telemetry-is-two-opt-ins`). `docs/rules/tooling/telemetry-is-two-opt-ins.md:1`, `crates/nvs-config/src/data.rs:459`
- [ ] The five verbs, and `on` says that nothing is uploaded while `endpoints.rs` names no upload URL. `crates/nvs-cli/src/telemetry.rs:1`
- [ ] A counter is `invocation.<subcommand>` from the command table, or `invocation.unknown`, and the argv never becomes a name (`rule:tooling/telemetry-counters-are-a-closed-set`). `crates/nvs-cli/src/telemetry.rs:1`, `crates/nvs-cli/src/main.rs:566`
- [ ] The payload is the JSON document the rule shows: `os` and `arch` mapped onto their closed lists, `days_active` from the day bits, and `first` from the lifetime upload count (`rule:tooling/telemetry-counters-are-a-closed-set`). `crates/nvs-cli/src/telemetry.rs:1`
- [ ] The panic hook counts `internal-error.<subcommand>` once per invocation, and a Novis program's own error counts nothing. `crates/nvs-cli/src/main.rs:502`, `crates/nvs-cli/src/telemetry.rs:1`
- [ ] `nvs telemetry show` prints the exact payload, and a test compares it with what a local listener receives (`rule:tooling/telemetry-show-prints-the-payload`). `crates/nvs-cli/src/telemetry.rs:1`
- [ ] `# nvs telemetry` in the CLI reference, after `# nvs info`: both consents, the payload field by field, every counter, `DO_NOT_TRACK`, and how to turn it off and delete the files (`rule:tooling/telemetry-show-prints-the-payload`). `docs/reference/tools/10-cli.md:509`
- [ ] Tests over a temporary data folder: consent round trip, `reset`, `DO_NOT_TRACK`, an unknown subcommand, `days_active` over a week of set days, `first` before and after an accepted upload, and a forced panic counted once. `crates/nvs-cli/src/telemetry.rs:1`
- [ ] The three rules `shipped`. `data/rules/tooling/telemetry-is-two-opt-ins.json:3`, `data/rules/tooling/telemetry-counters-are-a-closed-set.json:3`, `data/rules/tooling/telemetry-show-prints-the-payload.json:3`

### 4. an upload is carried, never fails the command, and never runs in a serving process

**Does:** the previous finished week's counters go out once, carried by a short-lived command, in
at most three seconds.

- [ ] The rule states the carrier allow-list as § *Standing decisions* settles it (`rule:tooling/a-serving-process-never-uploads`). `docs/rules/tooling/a-serving-process-never-uploads.md:1`, `crates/nvs-cli/src/main.rs:373`
- [ ] The weekly gate, the randomised point in the window, and `nvs telemetry upload`. `crates/nvs-cli/src/telemetry.rs:1`
- [ ] `nvs update-check` carries an upload when telemetry is on, over one connection when both are on. `crates/nvs-cli/src/update_check.rs:1`
- [ ] A three-second budget, one honest line on failure, and the command's own exit status untouched (`rule:tooling/a-failed-upload-never-fails-the-command`). `crates/nvs-cli/src/telemetry.rs:1`
- [ ] The answer decides the week's report: `204` records the upload, `429` and `5xx` keep it, `400`, `413` and `415` drop it, each with its line (`rule:tooling/a-failed-upload-never-fails-the-command`). `crates/nvs-cli/src/telemetry.rs:1`
- [ ] Tests against a local listener: one upload per finished week, none for an empty week, a dead listener inside the budget, and one case per status above. `crates/nvs-cli/src/telemetry.rs:1`
- [ ] The `nvs serve` test § *Standing decisions* names: zero connections under load, `invocation.serve` counted. `crates/nvs-cli/src/telemetry.rs:1`
- [ ] Both rules `shipped`. `data/rules/tooling/a-serving-process-never-uploads.json:3`, `data/rules/tooling/a-failed-upload-never-fails-the-command.json:3`

### 5. the project walk skips what `.gitignore` ignores, `nvs check` reads a directory, and a `require` that cannot be followed is an error

**Does:** every project walk goes through one function that reads the ignore files, and the two gap
records this stage owns are closed. Gate: `docs/goals/formatter-complete.md` no longer exists.

**Split:** one `Explore` subagent first lists every rule fragment under `docs/rules/` that says which
files a project walk, a workspace index or `nvs fmt` reads, as `file.md:NN` and one line each. Each
one is an item of the walk group. A second `Explore` subagent, sent with it, lists every `.nvs` and
`.nvst` file and every Rust test that writes a `require` whose path is not statically known. Each one
is rewritten in the `require` group.

The walk, one group:

- [ ] The `ignore` crate in the workspace `Cargo.toml` and in `nvs-hir`, and `bun nv lints` and `deny.toml` accept it. `Cargo.toml:1`, `crates/nvs-hir/Cargo.toml:17`
- [ ] `sources` walks with `WalkBuilder` and the settings § *Standing decisions* names, and its doc comment says so. `crates/nvs-hir/src/lenders.rs:239`
- [ ] `nvs fmt`'s walk is replaced by `sources`, and a path named on the command line is still formatted. `crates/nvs-cli/src/fmt.rs:177`
- [ ] Tests over a temporary tree: a file a `.gitignore` names, one an `.ignore` names, a nested `.gitignore`, no git repository, `vendor/` and a dot-directory. `crates/nvs-hir/src/lenders.rs:239`
- [ ] The rule names the ignore files, with the Declined exclude key (`rule:tooling/nvs-code-answers-about-the-project`). `docs/rules/tooling/nvs-code-answers-about-the-project.md:1`
- [ ] `rule:tooling/fmt-check-writes-nothing` says `nvs fmt` walks the project's files with the same function. `docs/rules/tooling/fmt-check-writes-nothing.md:1`
- [ ] Each fragment the subagent listed says the same. `docs/rules/tooling/nvs-code-answers-about-the-project.md:1`

`nvs check <directory>`, its own group:

- [ ] `nvs check` reads a directory through `Program::Directory` (`rule:testing/a-directory-of-programs-is-one-test-program`). `crates/nvs-cli/src/main.rs:1100`, `crates/nvs-cli/src/main.rs:2585`
- [ ] A test over a directory whose files have one error between them. `crates/nvs-cli/tests/check_command.rs:1`
- [ ] The gap record deleted. `data/gaps/nvs-cli/check-of-a-directory-is-not-one-program.json:1`

A `require` that cannot be followed, its own group:

- [ ] `rule:statements/require-is-the-only-inclusion-construct` lists the statically known forms and the new code. `docs/rules/statements/require-is-the-only-inclusion-construct.md:1`
- [ ] The new code beside `E0311`, with help text that follows AGENTS.md § *Text an end user reads*. `crates/nvs-diagnostics/src/code/names.rs:64`
- [ ] The dynamic fallback reports it at the `require`, and its module doc says so. `crates/nvs-hir/src/requires.rs:8`, `crates/nvs-hir/src/requires.rs:1837`
- [ ] A conformance case: a `require` of a call joined with a literal is the new error, and the literal form compiles. `tests/conformance/lang/a-require-whose-path-is-not-statically-known-is-a-compile-error.nvst:1`
- [ ] The gap record deleted, and `bun nv owners --closes cli-housekeeping` passes. `data/gaps/nvs-hir/a-require-path-that-is-not-static-reports-nothing.json:1`

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

Stages 1 and 2 are done. Stage 2 is `60dad5de8`. `crates/nvs-cli/src/endpoints.rs` reads
`NVS_UPDATE_URL` (default: the GitHub Releases API's latest release) and `NVS_TELEMETRY_URL` (no
default), and `nvs --help` lists both. `nvs update-check` is `crates/nvs-cli/src/update_check.rs`
over the new `nvs_stdlib::tool_get` (`crates/nvs-stdlib/src/http.rs`, beside `extension_send`). That
function is a Ctx-free `GET` over `transport::send`: system resolver, strict TLS, three redirects, no
plaintext hop, one attempt, no pooled connection, and a 1 MiB reply cap. Both rules are shipped. The
switch always prints `off` and the JSON's `"telemetry"` is the literal `"off"`, because telemetry
does not exist yet. A real run against GitHub prints `status 404, so no release is published`,
because the repository has no published release yet.

The tests written this session ran once and pass: `update_check::tests` (4). No test is expected to
go red.

The acceptance sweep's items are green at HEAD. `env-secret-proofs` passes. Of the nightly reds on
`7b6ef53ee`, these are fixed by later commits:

- the property-observer cost guard passes locally in release
- the proofs gate owes nothing
- `render --website --check` is current
- the shared-worker extension budget is `dc3a2ed29`
- the UNC alias is `93919e633`
- the macOS loopback is `342675093`
- the Windows `BUILTIN\Users` E0607 is `ba8930b80` and `8ea16803f`

Two memory-growth checks were not chased: `serve.rs`'s live-bytes guard on macOS, which
`config-follows-links` already names, and `serve_isolation.rs:614`'s flood on linux-aarch64. The next
nightly says whether either holds.

`crates/nvs-runtime/src/capability.rs` still has the uncommitted rustfmt join at line 2259. The guard
still refuses to stage it while interactive session 6884de4b claims the file.

### Next

**Stage 3: `nvs telemetry`**, one file set: `crates/nvs-cli/src/telemetry.rs` (new), the subcommand
table and the panic hook in `crates/nvs-cli/src/main.rs`, and the CLI reference. No gate.

- [ ] The rule states the paths and the `DO_NOT_TRACK`/`CI` switch. `docs/rules/tooling/telemetry-is-two-opt-ins.md:1`, `crates/nvs-config/src/data.rs:467`
- [ ] The five verbs, and `on` says nothing is uploaded while `endpoints::telemetry_url` is `None`. `crates/nvs-cli/src/telemetry.rs:1`, `crates/nvs-cli/src/endpoints.rs:28`
- [ ] One counter per invocation from the command table, with `invocation.unknown`. `crates/nvs-cli/src/main.rs:394`, `crates/nvs-cli/src/main.rs:606`
- [ ] The payload, `nvs telemetry show`, and the panic hook's `internal-error.<subcommand>`. `crates/nvs-cli/src/telemetry.rs:1`
- [ ] `nvs update-check` reads the real switch: replace the literal `off` in `check`, `upload_line` and `document`, and update its rule's **Not on disk.** line. `crates/nvs-cli/src/update_check.rs:67`, `crates/nvs-cli/src/update_check.rs:108`, `docs/rules/tooling/update-check-states-the-whole-picture.md:1`
- [ ] `# nvs telemetry` after `# nvs info`. `docs/reference/tools/10-cli.md:509`
- [ ] Tests over a temporary data folder as stage 3's last item lists, and the three rules shipped. `crates/nvs-cli/src/telemetry.rs:1`

### Backlog

- The telemetry collector and its default upload URL are goal `telemetry-service`'s work, which runs after this one (`docs/goals/telemetry-service.md`).
- `nvs dap` and `nvs debug` do not exist yet. Stage 4's allow-list leaves them out, so the goals that build them inherit it (`docs/goals/debugger-adapter.md`, `docs/goals/debug-service.md`).
