---
looper: 1
title: "nvs reload makes a running server match the disk, and says when it does"
summary: "Reached when nvs reload makes every nvs serve process of a data folder check its configuration and every loaded program at once, without settle, waits until each compile ends and exits 0, 1 with every error at its file and line, or 2 with no server; only the serving account or an administrator can reach it; every swapped or failed program is one log record and fires on = \"reload\" with programs and failed counts; and the deploy and development workflow is on the website, in the primer and in the feature's proofs."
context: [crates/nvs-cli/src/reload.rs, crates/nvs-cli/src/script.rs, docs/rules/config/an-edit-reaches-the-next-request-without-a-restart.md, docs/rules/config/the-config-is-an-immutable-snapshot.md]
stages:
  - id: 1
    title: the rules say what nvs reload does, and what it changes in the four rules it touches
    status: done
    context: [docs/rules/config/opcache-revalidation-is-system-class.md, docs/rules/config/an-entry-runs-on-a-server-event.md, docs/rules/packaging/a-service-answers-its-manager.md, docs/rules/config/a-reload-names-what-it-could-not-apply.md]
  - id: 2
    title: each nvs serve process opens a private reload endpoint in its data folder
    status: done
    context: [crates/nvs-config/src/data.rs, crates/nvs-config/src/trust.rs, crates/nvs-host/src/net.rs, crates/nvs-cli/src/serve.rs]
  - id: 3
    title: a reload through the endpoint checks now, skips settle, waits for its compiles and replies
    status: done
  - id: 4
    title: the nvs reload command
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/cache_cmd.rs]
  - id: 5
    title: a code swap is logged and fires on = "reload" with its counts
    status: open
    context: [crates/nvs-server/src/schedule.rs, crates/nvs-cli/src/serve.rs]
  - id: 6
    title: the feature's proofs, its help and the primer
    status: open
    context: [docs/reference/tools/25-server.md, docs/reference/README.md, crates/nvs-cli/src/agent.rs]
  - id: 7
    title: the website shows the deploy and development workflow, and lists the change
    status: open
    context: [website/src/content/docs/guides/production/index.mdx, website/src/content/docs/guides/production/run-as-a-service.mdx, website/src/content/docs/guides/production/docker.mdx]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: rules
    stage: 1
    name: "the rulebook validates with the new rule and the four amended ones"
    run: [bun, nv, rules, --check]
    timeout: 900
  - id: endpoint
    stage: 2
    name: "a serving process opens one private endpoint in its data folder, removes it on a stop, and a stale one of a dead process is skipped and removed"
    run: [cargo, test, --test, reload_command, --, the_endpoint_is_private_to_the_serving_account, a_stopped_server_removes_its_endpoint, a_stale_endpoint_is_skipped_and_removed, a_request_cannot_reach_the_endpoint]
    timeout: 1800
    tests:
      - the_endpoint_is_private_to_the_serving_account
      - a_stopped_server_removes_its_endpoint
      - a_stale_endpoint_is_skipped_and_removed
      - a_request_cannot_reach_the_endpoint
  - id: check-now
    stage: 3
    name: "a reload makes an edit live before the timer, applies a saved configuration at once, reports a broken edit with its line, and two reloads at once share one check"
    run: [cargo, test, --test, reload_command, --, a_reload_makes_an_edit_live_before_the_timer, a_reload_applies_a_saved_configuration_at_once, a_reload_reports_a_broken_edit_with_its_line, two_reloads_at_once_share_one_check, a_request_longer_than_the_line_is_closed]
    timeout: 1800
    tests:
      - a_reload_makes_an_edit_live_before_the_timer
      - a_reload_applies_a_saved_configuration_at_once
      - a_reload_reports_a_broken_edit_with_its_line
      - two_reloads_at_once_share_one_check
      - a_request_longer_than_the_line_is_closed
  - id: command
    stage: 4
    name: "nvs reload exits 0 when everything is live, 1 with every refusal, 2 with no server, reaches every server of one data folder, and --json prints one document"
    run: [cargo, test, --test, reload_command, --, nvs_reload_exits_zero_when_everything_is_live, nvs_reload_exits_one_and_prints_every_refusal, nvs_reload_exits_two_with_no_server, nvs_reload_reaches_every_server_of_a_data_folder, nvs_reload_json_is_one_document]
    timeout: 1800
    tests:
      - nvs_reload_exits_zero_when_everything_is_live
      - nvs_reload_exits_one_and_prints_every_refusal
      - nvs_reload_exits_two_with_no_server
      - nvs_reload_reaches_every_server_of_a_data_folder
      - nvs_reload_json_is_one_document
  - id: events
    stage: 5
    name: "a swapped program and a failed one are each one log record, a timer pass and nvs reload both fire on = \"reload\", and its args carry programs and failed"
    run: [cargo, test, --test, reload_command, --, a_swapped_program_is_one_log_record, a_failed_compile_is_one_log_record, a_code_swap_fires_the_reload_event_with_its_counts, a_timer_swap_fires_the_reload_event_too]
    timeout: 1800
    tests:
      - a_swapped_program_is_one_log_record
      - a_failed_compile_is_one_log_record
      - a_code_swap_fires_the_reload_event_with_its_counts
      - a_timer_swap_fires_the_reload_event_too
  - id: proofs
    stage: 6
    name: "nvs reload has every feature proof"
    run: [bun, nv, proofs, --id, "tools:server/nvs-reload", --gate]
    timeout: 1800
  - id: primer
    stage: 6
    name: "the primer tells an agent to run nvs reload after an edit"
    run: [cargo, test, --lib, --, the_primer_carries_the_reload_workflow]
    timeout: 1800
    tests:
      - the_primer_carries_the_reload_workflow
  - id: site-stale
    stage: 7
    name: "every page this goal reaches is reread and stamped"
    run: [bun, nv, site, --check, stale]
    timeout: 900
  - id: site-changes
    stage: 7
    name: "nvs reload, the reload event and the code-swap log records have their changes.json entries"
    run: [bun, nv, site, --check, changes]
    timeout: 900
  - id: site-review
    stage: 8
    name: "every user page this goal changed carries its newcomer review stamp"
    run: [bun, nv, site, --check, review]
    timeout: 900
---

# nvs reload makes a running server match the disk, and says when it does

## Why

A running server takes a code edit within `revalidate_freq` plus `settle` (2 s plus 1 s in
production, 2 s plus 100 ms in development), and a configuration edit within two seconds plus one
quiet check. Nothing tells a caller when that happened:

- **A deploy script** that runs a step after the update has to sleep and guess. A broken deploy is
  not seen at all: the requests that reach the broken program fail, and the script exits 0.
- **A coding agent** that edits files and then sends a request to a development server has to wait,
  or set `revalidate_freq = false`, which compiles every half-written state of a batch of edits.
- **No event fires for a code-only deploy.** `on = "reload"` fires only for configuration files, so
  the decline in `rule:config/an-entry-runs-on-a-server-event` ("a deploy or file-change event,
  which `reload` already is") is not true today. Nothing is logged when code is swapped either: the
  revalidation thread (`crates/nvs-cli/src/script.rs:1832`) logs nothing.

`nvs reload` is the step a deploy script and an agent run: the server checks now, and the command
returns once the new code is live, or names what did not compile.

## Standing decisions

The user decided the command, its events and the website workflow. The rest are the
authoring session's calls, made by the priority ordering; a session builds on them and does not
reopen them.

**One command, `nvs reload`, for code and configuration.** It makes a running server match the disk
now. It never empties a cache: units are keyed by content, so stale code is not possible, only a
delay, and emptying the table would recompile every program in front of live traffic.

**What a reload does, in order.** The configuration check runs at once, without the extra quiet check
a timed check needs (`crates/nvs-cli/src/reload.rs:530`), and publishes or refuses as today. Then every
loaded program is checked at once, with `settle` skipped: the caller says the writes are done. The
re-check after a compile stays, so a file that moved during the compile still discards it and
retries. The reply is sent when every compile the reload started has ended. A program no request has
loaded is not compiled; its first request compiles it, as today.

**The endpoint.** Each `nvs serve` process opens one endpoint under `<data>/run/`, a new
`Folder::run` beside `Folder::cache` (`crates/nvs-config/src/data.rs:155`), made private with the
same code as the data folder (`crates/nvs-config/src/data.rs:742`). Its name is
`serve-<pid>-<start time>`, so a restarted process never reuses one.
- **Unix:** a Unix socket at that path, in the private folder, and the peer's uid must be the serving
  account's or `0` (`SO_PEERCRED`, `getpeereid` on macOS), in `nvs-host`.
- **Windows:** a named pipe with a random 128-bit name, written into a file at that path. The pipe has
  a DACL for the serving account, `Administrators` and `SYSTEM`, `PIPE_REJECT_REMOTE_CLIENTS` and
  `FILE_FLAG_FIRST_PIPE_INSTANCE`, in `nvs-host`. No crate opens a named pipe today.
- **No TCP and no HTTP.** A request cannot reach it: the data folder is outside every grant, and a
  test proves it.
- **Idle cost:** one blocked accept on a thread the server already has, or one small thread, and
  nothing on the request path. The endpoint is removed in the drain. A command that finds the
  endpoint of a dead process skips it and removes it.

**The wire is one line each way.** The command sends `reload 1\n`. The server reads at most 64 bytes,
closes any other request without a reply, and answers with one JSON line: the configuration outcome
(`applied`, `refused` or `unchanged`), the restart keys, the swapped count and every refusal with its
file, line and message. The reply goes only to the serving account or an administrator, who can
read the log already, so it carries the error text. A constant, not a setting.

**Two reloads at once share one check.** A reply only ever comes from a check that began after the
reload arrived, because an earlier check may have read the files before the caller's writes. So a
reload that arrives while any check runs (a timer pass or another reload) waits for it, then one
more check runs, and every reload that arrived during the running one shares that check and its
reply: a burst costs at most two checks. (Stage 1 pinned this reading; the first wording's "or
starts one more check if the running one began before it arrived" held for every running check.)

**The command.** `nvs reload [--config <path>]... [--data <dir>] [--json]`. It resolves the data
folder as `nvs serve` does and reaches every endpoint there. Several servers that share one data
folder each get the reload, and the command reports each. Exit `0` when every server applied
everything, `1` when any configuration file or program was refused (the requests that reach a
refused program fail, `rule:config/a-broken-edit-fails-the-requests-that-resolve-it`), `2` when no
server answered. `--json` prints one document, in the shape of `nvs cache --json`. No `--timeout`:
compiles are bounded by their own limits, and a server that stops during a reload is reported as
stopped.

**Only `nvs serve`.** `nvs run` and `nvs test` start a new process that compiles from disk, so they
need no reload. A long `nvs run` worker is not reached.

**Service managers stay out.** No systemd `ExecReload` and no Windows control: the command is the
same on every platform, and both managers' units keep what
`rule:packaging/a-service-answers-its-manager` says, with one sentence naming `nvs reload`.

**No `never` for `revalidate_freq` or `validate`.** The documented way to rely on `nvs reload` alone,
for an agent or a developer, is a long interval such as `revalidate_freq = "1h"`. A forgotten reload
then still goes live within that interval.
`rule:config/opcache-revalidation-is-system-class` keeps its decline of `never` and gains this
sentence.

**Logging.** One `Core\Log` record per swapped program (`program reloaded`: the entry file and its
new program digest) and one per program that does not compile (`program reload failed`: the entry
file, and the first diagnostic's file, line and message). Both are written by the revalidation path
in `crates/nvs-cli/src/script.rs` whether a timer or `nvs reload` started the check. Cost: one record
per edit, never per request.

**The event.** No new `on` value. Every check pass that swapped or refused a program, and every
reload, is one reload outcome in `Events` (`crates/nvs-server/src/schedule.rs:331`). The args map
(`crates/nvs-cli/src/serve.rs:2171`) adds `"programs" => <swapped count>` and
`"failed" => <refused program count>`. `outcome` is `refused` when the configuration or any program
was refused, and `applied` otherwise. The map carries no path and no error text, as today. The ticker
already merges outcomes between two passes, so a fast-editing agent starts no burst of runs. The
decline in `rule:config/an-entry-runs-on-a-server-event` then holds as written.

**Where the feature lives.** A `# nvs reload` section in `docs/reference/tools/25-server.md` after
`# What reaches a running server` (`docs/reference/tools/25-server.md:318`), which makes the feature
`tools:server/nvs-reload`. It is marked `<!-- primer -->`, so `nvs agent primer` tells an agent to run
`nvs reload` after an edit (`rule:tooling/a-primer-claim-is-executed`).

**The website.** A new guide page `website/src/content/docs/guides/production/deploy.mdx`, *Deploy a
new version*: switch a `current` link to the finished release, run `nvs reload`, check its exit code,
and run migrations with `nvs run`, which compiles from disk at once. It has a section for development
and coding agents: `revalidate_freq = "1h"` and `nvs reload` after each batch of edits. The
production index, the service page and the Docker page link to it.

**The endpoint's shape, as stage 2 built it.** It is `nvs_host::endpoint` (`crates/nvs-host/src/endpoint.rs`,
with `endpoint/unix.rs` and `endpoint/windows.rs`), not a type in `net.rs`: it blocks on a thread of
its own and never parks on a core, so it shares nothing with the parking streams. The path appears
only once it can be reached (bound under `.new`, then renamed), so a path nobody listens at is
stale for certain. **A starting server removes every stale endpoint in `run/`** (`reach_all`), and
the command does the same; that is what a killed server leaves. **A Unix socket path over
`sun_path` gets no endpoint**: the server warns once and serves, and a shorter `--data` is the fix.
No second socket folder exists to keep private and track. The Windows pipe name's 128 bits come
from `ring::rand::SystemRandom`, already in the binary. The accepted Unix stream has a 5 s read and
write timeout; the Windows stream has none yet.

**The reply, as stage 3 built it.** One JSON line:
`{"config":"applied|refused|unchanged","restart":[...],"programs":N,"failed":N,"refusals":[...]}`.
Each refusal is `{"kind":"configuration|program","entry":<entry file or null>,"file","line","message"}`,
with the first error's file, line from 1 and one-line message; `line` is `null` and `file` is the
entry where the error has no place in a file (a backend error, a panic). `failed` counts every
loaded program that does not compile now, not only those this check compiled: a reload reports
every program that does not match the disk, so `nvs reload` exits `1` while one stays broken. A
program whose tree still moves after three compiles is reported failed with "changed while it was
compiled". A draining server closes the connection without a reply, which the command reports as
stopped. The first error reaches the table through `Sink::Serve` (`crate::take_first_error`), a
thread-local the compiling thread reads right after its front end, and the configuration's through
`reload::rendered`'s `REFUSED_AT`: both keep the shapes every other caller uses (`Answer` and
`Result<Report, String>`) unchanged. A timer pass runs beside a reload: the configuration half
shares the one reload lock, and a program both compile is single-flighted, so `Rounds` orders only
reloads among themselves.

**Lanes.** Stage 1 touches only rules. Stage 2 (`nvs-config`, `nvs-host`, `serve.rs`) and stage 5
(`schedule.rs`, `run_args`) are disjoint, but stage 5's tests drive `nvs reload`, so the stages run in
order. One goal, no side lanes.

**What stops it.** One attempt per item and two sessions per stage. A stage that does not close in two
sessions writes what holds it in the handoff and the next session splits it.

## Stages

### 1. the rules say what nvs reload does, and what it changes in the four rules it touches

- [x] `docs/rules/config/a-reload-command-checks-now-and-waits.md:1` and
  `data/rules/config/a-reload-command-checks-now-and-waits.json:1`: the new rule, from § *Standing
  decisions*: what a reload does in order, the endpoint and who can reach it, the wire, two at once,
  the command and its exit codes, only `nvs serve`, and its costs. Declined: a cache flush; a TCP or
  HTTP endpoint; `SIGHUP`; a systemd `ExecReload` or SCM control; a `--timeout`; a `never` value.
- [x] `docs/rules/config/an-edit-reaches-the-next-request-without-a-restart.md:40`: "a client cannot
  trigger a recompile" names the new rule as the one local exception, and `settle` is skipped only by
  it.
- [x] `docs/rules/config/the-config-is-an-immutable-snapshot.md:47`: "Nothing else starts a reload"
  names `nvs reload`. The `SIGHUP` decline keeps its reasons and says a socket answers them.
- [x] `docs/rules/packaging/a-service-answers-its-manager.md:9`: one sentence that `nvs reload` is the
  way to reload, on every platform, and a unit has no reload command.
- [x] `docs/rules/config/opcache-revalidation-is-system-class.md:9`: the long interval as the way to
  rely on `nvs reload` alone.
- [x] `docs/rules/config/an-entry-runs-on-a-server-event.md:38`: `reload` fires for a code pass too,
  with `programs` and `failed`, and `outcome` covers both.
- [x] `docs/rules/config/a-reload-names-what-it-could-not-apply.md:1`: the two program records.
- [x] `bun nv rules --render`, then the `rules` check.

### 2. each nvs serve process opens a private reload endpoint in its data folder

**Split:** the Windows named pipe and its DACL in `crates/nvs-host/src/` go to one writing subagent,
the Unix socket and peer check to another; the session wires `serve.rs` and builds once both end.

- [x] `crates/nvs-config/src/data.rs:155`: `Folder::run`, made private as `prepare` makes the folder
  (`crates/nvs-config/src/data.rs:719`).
- [x] `crates/nvs-host/src/net.rs:717`: a reload endpoint type with `bind`, `accept` and `connect`,
  the Unix peer-uid check and the Windows pipe with its DACL and name file.
- [x] `crates/nvs-cli/src/serve.rs:2961`: open the endpoint before `READY=1`, remove it in the drain.
- [x] `crates/nvs-cli/tests/reload_command.rs:1`: the four `endpoint` tests. Run them in WSL too with
  `bun nv linux-test`.

### 3. a reload through the endpoint checks now, skips settle, waits for its compiles and replies

- [x] `crates/nvs-cli/src/reload.rs:530`: a check-now path into `noticed` without the extra quiet
  check, returning the outcome.
  - [x] `reread` and `reported` shared by `noticed` and `reload_now`.
  - [x] `Configured`, with the refusal's place kept by `rendered` (`REFUSED_AT`).
- [x] `crates/nvs-cli/src/script.rs:934`: a `revalidate_now` that skips `unsettled`'s wait
  (`crates/nvs-cli/src/script.rs:1123`), compiles through the same single-flight path, and returns
  each swap (`advance`, `crates/nvs-cli/src/script.rs:1382`) and failure (`fail`,
  `crates/nvs-cli/src/script.rs:1353`).
  - [x] `CompileState::Failed` keeps the first error (`Failure`), through `Sink::Serve`.
  - [x] `revalidate_now` and `Rechecked`, with up to three tries for a tree that moves.
- [x] `crates/nvs-cli/src/serve.rs`: the endpoint's accept loop reads the line, runs one shared
  check, and writes the reply.
  - [x] One thread per connection, at most 16, and the strict line read.
  - [x] `Rounds`: a reload is answered by a check that began after it, and waiters share one.
  - [x] The JSON reply, and three unit tests.
- [x] `crates/nvs-cli/tests/reload_command.rs`: the five `check-now` tests, each with
  `revalidate_freq = "1h"` so only the reload can be the cause.
  - [x] Green on Windows and in WSL; the rule's record is `shipped`.

### 4. the nvs reload command

- [ ] `crates/nvs-cli/src/reload_cmd.rs:1`: the command, its help text (AGENTS.md § *Text an end user
  reads*), the text and `--json` output, and the exit codes.
- [ ] `crates/nvs-cli/src/main.rs:577`: `Reload(reload_cmd::Args)` in `Command`, beside `Cache`
  (`crates/nvs-cli/src/main.rs:595`).
- [ ] `crates/nvs-cli/tests/reload_command.rs`: the five `command` tests.

### 5. a code swap is logged and fires on = "reload" with its counts

- [ ] `crates/nvs-cli/src/script.rs:1382` and `crates/nvs-cli/src/script.rs:1353`: the two log
  records, through the writer `crates/nvs-cli/src/reload.rs:605` uses.
- [ ] `crates/nvs-server/src/schedule.rs:263`: `Reloaded` gains the two counts, and a code pass
  calls `Events::reloaded` (`crates/nvs-server/src/schedule.rs:331`).
- [ ] `crates/nvs-cli/src/serve.rs:2171`: `run_args` adds `programs` and `failed`.
- [ ] `crates/nvs-cli/tests/reload_command.rs`: the four `events` tests.

### 6. the feature's proofs, its help and the primer

**Split:** the examples and the attack go to one writing subagent once the reference section is
written.

- [ ] `docs/reference/tools/25-server.md:318`: the `# nvs reload` section, marked `<!-- primer -->`,
  and `# What reaches a running server` names it.
- [ ] `docs/examples/tools/server/nvs-reload/about.md:1`, and the examples
  `bun nv proofs --id tools:server/nvs-reload` names, among them a deploy script and an edit made
  live at once with `revalidate_freq = "1h"`.
- [ ] `tests/hostile/tools/server/nvs-reload/01-a-request-tries-to-reload-its-own-server.nvs:1`: a
  request tries to reach the endpoint through the file system and the network, and every attempt
  fails.
- [ ] `docs/examples/tools/server/what-reaches-a-running-server/about.md:1` and
  `docs/examples/config/schedule/`: name `nvs reload` and the new event keys.
- [ ] `crates/nvs-cli/src/agent.rs`: `the_primer_carries_the_reload_workflow`.
- [ ] `bun nv proofs --comments` over every new `.nvs`, then the `proofs` check.

### 7. the website shows the deploy and development workflow, and lists the change

- [ ] `website/src/content/docs/guides/production/deploy.mdx:1`: *Deploy a new version*, as § *Standing
  decisions* says, with its development and agent section.
- [ ] `website/src/content/docs/guides/production/index.mdx`,
  `website/src/content/docs/guides/production/run-as-a-service.mdx` and
  `website/src/content/docs/guides/production/docker.mdx:69`: link to it, and `docker exec <c> nvs
  reload`.
- [ ] `website/src/data/changes.json`: `new` `nvs reload`, `changed` the `on = "reload"` event, `new`
  the two log records.
- [ ] Reread and stamp every page `bun nv site --check stale` names.

### 8. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of every page this goal changed ([prose.md](../agent/prose.md) § *A
  newcomer reads it*).
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 3 are done; `rules`, `endpoint` and `check-now` are green. All nine
`reload_command` tests pass on Windows and in WSL (`bun nv linux-test --test reload_command`),
and the three `reload_endpoint::tests` unit tests pass. macOS is still uncompiled here; CI's macOS
job is its first build. `rule:config/a-reload-command-checks-now-and-waits` is `shipped`, with
`reload_endpoint.rs` and the test file in its `guardedBy`.

The server answers `reload 1\n` with the reply § *Standing decisions* § *The reply, as stage 3
built it* describes. `Rechecked::Swapped`'s two fields carry an `#[expect(dead_code)]` until
stage 5's `program reloaded` record reads them; stage 5 removes it.

The nightly CI run the sweep listed (44722386375) ran on `5d053fc6e6`, the same run the last two
handoffs covered; its failures were fixed after it, so no change was made for them.

### Next

Stage 4, the command. The client side is `nvs_host::endpoint::reach_all`
(`crates/nvs-host/src/endpoint.rs`), which returns each live endpoint's connection and removes the
stale ones. Send `crate::reload_endpoint::REQUEST` (`crates/nvs-cli/src/reload_endpoint.rs:59`),
read one line, and parse the JSON `reply` writes (`crates/nvs-cli/src/reload_endpoint.rs:194`). A
connection that closes with no line is a server that stopped (it was draining). Exit `0` when every
reply is `failed: 0` and not `config: refused`, `1` otherwise, `2` when no endpoint answered. The
test helper `reload` in `crates/nvs-cli/tests/reload_command.rs` is the client shape. The data
folder resolves as `nvs serve`'s does (`nvs_config::data::current`, `Folder::run`). Wire it as
`Command::Reload` beside `Command::Cache` (`crates/nvs-cli/src/main.rs:596`, `:560`, `:954`).
The `command` and `events` checks stay red until stages 4 and 5 write their tests.

Stage 5 then logs from `Compiler::revalidate_now`'s and `revalidate`'s results
(`crates/nvs-cli/src/script.rs:1012`) and feeds `Events` beside `Process::logged`
(`crates/nvs-cli/src/reload.rs:687`).

### Backlog
