---
looper: 1
title: Every block of code the website shows is checked by a real run
summary: "Reached when bun nv site --check code fails on any code block of a hand-written page or an example app that no check runs and that no marker exempts with a reason, and is green."
context: [tools/nv/cmd/site.ts, tools/nv/cmd/reference.ts, website/README.md, .github/workflows/ci.yml]
stages:
  - id: 1
    title: "every code block of the website is listed with what checks it, and the code part exists"
    status: done
  - id: 2
    title: "every TOML, JSON and YAML block is parsed by the tool that reads it"
    status: done
  - id: 3
    title: "every shell and output block is a run, a parsed nvs command, or marked with its reason"
    status: done
  - id: 4
    title: "every nginx, Caddy and Dockerfile block, and every app's deploy/ folder, passes its own validator"
    status: done
  - id: 5
    title: "the code part runs in CI and in bun nv ci --check, and its rule is written"
    status: done
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: code-part
    stage: 5
    name: every code block on the website is checked or carries its reason
    run: [bun, nv, site, --check, code]
    timeout: 1800
---

# Every block of code the website shows is checked by a real run

## Why

The user decided on 2026-11-05: "every code on the website needs real checks". A reader copies what
the website shows, so a block that no run checks is a block that can be wrong without anything
turning red. Two cases from that day: Paperwork's stored PDFs went stale for weeks because
`bun nv site --check apps` ran nowhere automatic (fixed in `b2864a13e1`), and the reverse proxy
guide's Unix-socket configuration left every browser form and WebSocket failing with `403`
(fixed in `d1ad6c75ce`), which no check could see because the block was never run.

What is checked today: `<Snippet>` and `<Terminal>` blocks run against their `.out`
(`bun nv site --check snippets`), `<Shot>` pictures are taken again (`shots`), ```` ```nvs ```` blocks
of the reference chapters run (`bun nv reference`, which honours ```` ```nvs skip ````), the `Core`
pages' examples run as feature proofs, and every example app's `nvs test .`, server and stored files
run (`apps`). What is not: about 140 fenced blocks written straight into the hand-written pages under
`website/src/content/docs/`. On 2026-11-05 they were 65 `sh`, 34 `toml`, 21 `txt`, 4 `json`,
4 `bat`, 2 `yaml`, 2 `nginx`, 2 `caddyfile` and one each of `rust`, `powershell`, `md` and
`dockerfile`. Each app's `deploy/` files are not checked either: `Caddyfile`, `nginx.conf` and
`production.toml`.

## Standing decisions

1. **The rule:** every code block on a hand-written page, and every file of an example app the
   website shows, is either run by a check or carries a marker that says why it is not. The marker
   is the fence's meta: ```` ```sh skip="installs a system service" ````, the same place
   ```` ```nvs skip ```` already uses (`tools/nv/cmd/reference.ts:510`). A `skip` with no reason is a
   failure. Stage 5 writes this as the rule `testing/every-code-block-on-the-website-is-checked`.
2. **One new part, `code`,** in `tools/nv/cmd/site.ts` beside `apps` and `snippets`. It walks every
   hand-written page (the pages no renderer writes, which `site.ts` already lists for `covers:`),
   reads each fence's language and meta, runs the language's check, and names each block it could not
   check as `page:line`. It reaches CI only in stage 5, once it is green, so no stage leaves CI red.
3. **What checks each language:**
   - `toml`: the block is written to a scratch file and read by `nvs config check`. A block that shows
     one table is a whole file, so it parses alone. A block that is only part of a file says so with
     `fragment` in its meta, and is checked inside the smallest file that holds it.
   - `json`: parsed. `yaml`: parsed by `Bun.YAML.parse`.
   - `sh`: each line that runs `nvs` or `bun nv` is parsed by the command's own argument parser
     without running it. The session decides the mechanism; the safe default is an environment
     variable the CLI reads before it dispatches, which exits 0 once the arguments parse. Every other
     line (`git`, `usermod`, `systemctl`) is the reason the block carries `skip="..."`, unless the
     block becomes a `<Terminal>` that really runs.
   - `txt`: a program's output becomes the `.out` of a `<Snippet>` or `<Terminal>` that produces it.
     A file tree or a table of text carries `skip="a file tree"`.
   - `nginx` and `caddyfile`: validated by the proxy itself, in its official image pinned by digest,
     the way `bun nv pdf-validate` runs veraPDF (`tools/nv/cmd/pdf-validate.ts`): `nginx -t` with
     stub certificate files at the paths the block names, and `caddy validate --adapter caddyfile`.
     Linux only, as veraPDF is, because the other legs' Docker runs no Linux containers.
   - `dockerfile`: `docker build --check`, in the same Linux step.
   - `bat`, `powershell`, `rust`, `md`: the session decides each by what it costs. `rust` is the
     extension guide's project, which `bun nv ext-templates --example` already builds; the block is
     checked as a copy of that project's file.
4. **An app's `deploy/` folder** is checked by the `apps` part: `nvs config check --config nvs.toml
   --config deploy/production.toml` from the app's folder, and its `Caddyfile` and `nginx.conf` by the
   same proxy validators. A production path that does not exist on the machine that checks
   (`/var/lib/chat/`) does not fail the check; if `config check` says it does, the session decides
   how the check states that, by the priority ordering.
5. **The gate with `realtime-example-apps`:** stage 4 starts once that goal's stage 1 has made
   `apps/chat/deploy/` and `apps/poll/deploy/` one shape, which `apps/chat/deploy/server.toml` being
   gone proves. Until then stage 4 checks only the pages' own blocks.
6. **Fixing what the checks find** is part of each stage: a block that fails its check is corrected
   on the page, never marked `skip`.
7. **Lanes:** stage 1 first. Stages 2, 3 and 4 add one language each to the `code` part and touch
   different pages, so they may run as side goals once stage 1 is committed. Stage 5 is last.
8. **The other languages:** a `bat` or `powershell` block is a command line, so stage 3 checks it
   the way it checks `sh`: each `nvs` line is parsed, and a block with any other command gets
   `skip="..."`. Stage 4 checks the `rust` block as a copy of the extension example's file
   (decision 3). It checks the `md` block, which shows a README, the same way, or marks it. A new
   language is added to `CODE_CHECKS` under its own name. `CODE_ALIASES` maps the other names a
   fence may use (`bash` is `sh`, `yml` is `yaml`, `caddy` is `caddyfile`).
9. **A file a `toml` block names that the checking machine does not have** gets a stub, and the
   block is checked again (`tomlProblems`). `nvs config check` stops at the first `E0605` and hides
   every other error, so tolerating the error would leave the rest of the block unchecked. Adding a
   flag to the CLI that skips file reads would weaken the command for everybody, and the fix is only
   needed here. A relative path's stub goes in the block's scratch folder. An absolute path, such as
   `/run/secrets/api_key`, is first rewritten under that folder, because nothing writes outside the
   repository. A stub has one line, since an empty secret file is its own error (`E0608`).
   `fragment` is per language: `fragment="<table>"` on `toml` puts the block under that header, and
   a bare `fragment` on `json` parses it inside `{ }`, for a VS Code setting or one OpenAPI path.
   No `toml` page block needs `fragment` today.
10. **How a shell and an output block are checked.** `NOVIS_PARSE_ONLY` (`crates/nvs-cli/src/main.rs`)
    makes `nvs` exit 0 once clap accepts its arguments, before elevation, the data folder and
    telemetry, and it is in `nvs --help`'s environment list. A `bun nv` line is checked against the
    command table in `tools/nv/main.ts`, which cannot be imported without running. A `skip` skips the
    whole block, so a block that mixes `cargo` and `nvs` lines checks neither: splitting it on the page
    would cost the reader a step for one parse. A `txt` block is checked in place with
    `run="nvs ..."` and `cwd`, rather than turned into a `<Terminal>`: a `.term`'s fixture is a copy,
    and the apps and snippets the output comes from would then have two homes. Its lines must appear
    in the output in order, so a page may show an excerpt, and an error the reader makes by changing
    a program is `skip`.
11. **How the proxy, Docker, `rust` and `md` blocks are checked.** `bun nv proxy-validate`
    (`tools/nv/cmd/proxy-validate.ts`) is the one validator, for the `code` part, the `apps` part and
    a person. The text reaches the container on standard input, so nothing is mounted. A Caddyfile
    that needs a plugin says so with `plugins="<module>"`. Each plugin is pinned in `CADDY_PLUGINS`,
    and Caddy is built with it by `xcaddy` into a local image tagged by its Dockerfile's hash. That is
    a build of about a minute, once per machine, and it keeps the page's real `rate_limit` block
    checked. Removing the block from the page would cost the reader the login limit. Where Docker runs
    no Linux containers, the proxy checks print why once and pass, as `pdf-validate` does, so the
    Linux leg is the one that judges them. A `rust` block names its source with `copy="<path>"`, and
    its lines must be a run of that file's lines. The `md` block is text the reader types into the
    demo page, so it is `skip`. `nvs config check nvs.toml deploy/production.toml` accepts
    `/var/lib/<app>/` paths, so decision 4's stub is not needed.
12. **The proxy checks run inside CI's website stale guard step, not in a step of their own.** The
    `code` and `apps` parts call `bun nv proxy-validate` themselves, and print why they pass where
    Docker runs no Linux containers, so a second Linux step beside veraPDF would run the same checks
    twice. The step's job has a 90-minute limit, so Caddy's one-minute plugin build fits.

## Stages

### 1. every code block of the website is listed with what checks it, and the code part exists

**Does:** turns the counts in *Why* into a list, and lands the part that later stages fill.

**Split:** a `reader` subagent lists every fenced block of every hand-written page and every file of
`apps/*/deploy/`, as `page:line language meta`, with the check that runs it today or `none`.

- [x] The list, written into this goal's `### Next` as the work of stages 2 to 4.
  `website/src/content/docs/guides/production/reverse-proxy.mdx:1`
- [x] `site --check code` exists, reads each fence's language and meta, fails on a `skip` with no
  reason, and names every block no check runs. It is not yet in CI. `tools/nv/cmd/site.ts:318`
- [x] A Bun test for the part over a fixture page with each kind of fence. `tools/nv/test/site.test.ts:1`

### 2. every TOML, JSON and YAML block is parsed by the tool that reads it

- [x] The `toml` check with `nvs config check`, and `fragment` for a partial block (decision 3).
  `tools/nv/cmd/site.ts:318`
- [x] The `json` and `yaml` checks. `tools/nv/cmd/site.ts:318`
- [x] Every block the checks name is corrected on its page.
  `website/src/content/docs/guides/how-to-use/configuration.mdx:1`

### 3. every shell and output block is a run, a parsed nvs command, or marked with its reason

- [x] The argument parse without a run for `nvs` and `bun nv` lines (decision 3).
  `crates/nvs-cli/src/main.rs:1`
  - [x] `NOVIS_PARSE_ONLY` in `main`, after the parse, and in the help's environment list.
  - [x] A `bun nv` line names a command of `tools/nv/main.ts`.
- [x] The `sh` check over it, and each `sh` block made a `<Terminal>` or marked.
  `website/src/content/docs/guides/production/run-as-a-service.mdx:1`
  - [x] `shellCommands` and `shellProblems`, for `sh`, `bat` and `powershell`, with tests.
  - [x] The 28 blocks with other programs carry `skip="..."`.
- [x] Each `txt` block made the `.out` of a run or marked. `website/src/content/docs/guides/example-apps/notes.mdx:58`
  - [x] `outputProblems` with `run` and `cwd` (decision 10), with a test.
  - [x] Seven blocks run, and the trees, path lists and one-server output carry `skip="..."`.

### 4. every nginx, Caddy and Dockerfile block, and every app's deploy/ folder, passes its own validator

**Gate:** `apps/chat/deploy/server.toml` is gone (decision 5).

- [x] A command that validates nginx and Caddy files in their pinned images, on the model of
  `tools/nv/cmd/pdf-validate.ts:1`, used by the `code` part on Linux.
  - [x] `bun nv proxy-validate` with `nginx -t`, `caddy validate` and `docker build --check`.
  - [x] Caddy plugins pinned and built once per machine (decision 11).
  - [x] Its Bun test, which starts no Docker.
- [x] The pages' `nginx`, `caddyfile` and `dockerfile` blocks pass, corrected where they fail.
  `website/src/content/docs/guides/production/reverse-proxy.mdx:1`
  - [x] `proxyProblems` and `copyProblems` in `CODE_CHECKS`.
  - [x] The Caddyfile names its plugin, the `rust` block its file, and the `md` block is `skip`.
- [x] The `apps` part runs `nvs config check` over each `deploy/production.toml`, and the proxy
  validators over each `deploy/Caddyfile` and `deploy/nginx.conf` (decision 4).
  `tools/nv/cmd/site.ts:1913`

### 5. the code part runs in CI and in bun nv ci --check, and its rule is written

- [x] `code` in CI's website stale guard step, and the proxy validation in a Linux step beside
  veraPDF (decision 12). `.github/workflows/ci.yml:625`
- [x] `code` in `PRE_PUSH`'s stale guard. `tools/nv/cmd/ci.ts:240`
- [x] The rule `testing/every-code-block-on-the-website-is-checked`, rendered, and named in
  `website/README.md`'s table of who owns which file. `docs/rules/testing/in-process-request.md:1`

### 6. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Every stage is done, and the goal is reached. `looper verify` failed once, because
`docs/rules/testing.md` was stale after decision 12 went into the rule's Declined list. It was
re-rendered, and the second run is green. `bun nv verify --doc` is green. `bun nv owners --closes
website-code-checked` names no gap. `bun nv ci --check` was red on `site questions`: 19 pages
that earlier stages gave fence meta (`skip=`, `run=`, `fragment`, `plugins=`) were stale. Their
questions already matched each page, so they were stamped, and `ci --check` is green, `site code`
included.

Decision 2 and decision 8 live in `tools/nv/cmd/site.ts`'s header (§ *A code block*) and in the
rule. Decision 12 is now in the rule's Declined list. Decision 7 only ordered this goal's stages.

The sweep's reds were stale. No `env-secret-proofs` check or log exists. The nightly Miri reds ran
a commit older than `f13f40646a`, which skips the three 8 MiB JSON tests under Miri. No test is
expected to go red.

### Next

Nothing in this goal. The driver runs the `code-part` acceptance check and retires the goal.

### Backlog

- `bun nv site --check apps` left `apps/notes/notes.sqlite-shm` and `notes.sqlite-wal` behind once,
  beside an ignored `notes.sqlite` from an older run. `inApp` (`tools/nv/cmd/site.ts`, search
  `async function inApp`) should delete them. Find out whether `filesUnder` skips ignored files, or
  whether a process still holds them when the cleanup runs on Windows.

- From `realtime-example-apps`: a connection's `cpu_time` counts from the upgrade for its whole life,
  so poll's production `cpu_time = "5s"` may end a busy event stream after hours. Check whether the
  watchdog charges an unwatched connection isolate (`crates/nvs-host/src/watchdog.rs:1348`) and
  whether a connection wants a per-message budget (`rule:concurrency/a-connection-is-a-root-isolate`).
- From `realtime-example-apps`: whether `connect-src 'self'` allows `wss:` to the same host in every
  supported browser. The Chromium test covers `ws:` over `http:` only (`apps/chat/nvs.toml:14`).
- From `realtime-example-apps`: a screenshot of each app on its page, once `bun nv site --shoot` has
  an app shot (`tools/nv/cmd/site.ts:200`).
