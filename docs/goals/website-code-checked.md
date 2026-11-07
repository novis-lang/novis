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
    status: open
  - id: 3
    title: "every shell and output block is a run, a parsed nvs command, or marked with its reason"
    status: open
  - id: 4
    title: "every nginx, Caddy and Dockerfile block, and every app's deploy/ folder, passes its own validator"
    status: open
  - id: 5
    title: "the code part runs in CI and in bun nv ci --check, and its rule is written"
    status: open
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
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

- [ ] The `toml` check with `nvs config check`, and `fragment` for a partial block (decision 3).
  `tools/nv/cmd/site.ts:318`
- [ ] The `json` and `yaml` checks. `tools/nv/cmd/site.ts:318`
- [ ] Every block the checks name is corrected on its page.
  `website/src/content/docs/guides/how-to-use/configuration.mdx:1`

### 3. every shell and output block is a run, a parsed nvs command, or marked with its reason

- [ ] The argument parse without a run for `nvs` and `bun nv` lines (decision 3).
  `crates/nvs-cli/src/main.rs:1`
- [ ] The `sh` check over it, and each `sh` block made a `<Terminal>` or marked.
  `website/src/content/docs/guides/production/run-as-a-service.mdx:1`
- [ ] Each `txt` block made the `.out` of a run or marked. `website/src/content/docs/guides/example-apps/notes.mdx:58`

### 4. every nginx, Caddy and Dockerfile block, and every app's deploy/ folder, passes its own validator

**Gate:** `apps/chat/deploy/server.toml` is gone (decision 5).

- [ ] A command that validates nginx and Caddy files in their pinned images, on the model of
  `tools/nv/cmd/pdf-validate.ts:1`, used by the `code` part on Linux.
- [ ] The pages' `nginx`, `caddyfile` and `dockerfile` blocks pass, corrected where they fail.
  `website/src/content/docs/guides/production/reverse-proxy.mdx:1`
- [ ] The `apps` part runs `nvs config check` over each `deploy/production.toml`, and the proxy
  validators over each `deploy/Caddyfile` and `deploy/nginx.conf` (decision 4).
  `tools/nv/cmd/site.ts:1913`

### 5. the code part runs in CI and in bun nv ci --check, and its rule is written

- [ ] `code` in CI's website stale guard step, and the proxy validation in a Linux step beside
  veraPDF. `.github/workflows/ci.yml:625`
- [ ] `code` in `PRE_PUSH`'s stale guard. `tools/nv/cmd/ci.ts:240`
- [ ] The rule `testing/every-code-block-on-the-website-is-checked`, rendered, and named in
  `website/README.md`'s table of who owns which file. `docs/rules/testing/in-process-request.md:1`

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 is done (`eccf3367c3`). `bun nv site --check code` exists and is not in CI. It reads every
fenced block of every handwritten page, and today it fails on 144 blocks because `CODE_CHECKS`
(`tools/nv/cmd/site.ts`, search `export const CODE_CHECKS`) is still empty. Each later stage
registers one language's check there as a `CodeCheck`. A check gets the block (its text, language
and attributes such as `fragment`) and `{ root, nvs }`, and returns its problems without `page:line`.
The part's four tests pass (`bun test tools/nv/test/site.test.ts -t "check code"`). No page has an
`nvs` fence. No `about.md` or `details.md` under `docs/examples/` has a fence of any kind, so the
`Core` cards add no blocks. No test is expected to go red.

### Next

Stages 2, 3 and 4 are lanes over different pages (decision 7), and any of them may go next. Stage
4's gate is open: `apps/chat/deploy/` has `Caddyfile`, `nginx.conf` and `production.toml`, and no
`server.toml`. Each list below is `page line, ...` under `website/src/content/docs/`, which is what
`site --check code` names now.

**Stage 2: `toml` (36), `json` (4) and `yaml` (2).** `tools/nv/cmd/site.ts` (`CODE_CHECKS`)
- `toml`: `guides/how-to-use/configuration.mdx` 41, 86, 102, 114, 134, 150, 177, 187;
  `guides/production/reverse-proxy.mdx` 39, 46, 79, 93, 410, 477; `guides/production/limits-and-logs.mdx`
  16, 35, 44, 61; `guides/how-to-use/limits.mdx` 28, 38, 55; `guides/production/docker.mdx` 196, 288,
  332; `guides/cookbook/page-db-rows.mdx` 15; `guides/cookbook/schedule-job.mdx` 15;
  `guides/cookbook/upload-file.mdx` 16; `guides/extensions/rust.mdx` 248;
  `guides/how-to-use/live-updates.mdx` 88; `guides/production/run-as-a-service.mdx` 96;
  `guides/simple-programs/scheduled-job.mdx` 16; `install/server.mdx` 183; `tools/cli/index.mdx` 67;
  `tools/debug-viewer/index.mdx` 20; `tools/editor/index.mdx` 99; `tools/profiler/index.mdx` 75.
- `json`: `syntax/attributes/routes.mdx` 63; `tools/debugger/index.mdx` 20; `tools/editor/index.mdx`
  202, 222.
- `yaml`: `guides/production/docker.mdx` 208, 300.

**Stage 3: `sh` (66), `bat` (5), `powershell` (1) and `txt` (23).** `crates/nvs-cli/src/main.rs:1`
- `sh`: `guides/example-apps/chat.mdx` 42, 51, 64, 251; `guides/example-apps/notes.mdx` 32, 41, 50,
  144; `guides/example-apps/paperwork.mdx` 57, 66, 81; `guides/example-apps/poll.mdx` 38, 47, 59, 177,
  237, 247, 254, 262; `guides/extensions/rust.mdx` 35, 43, 62, 189, 209, 243, 257, 309;
  `guides/how-to-use/configuration.mdx` 120; `guides/production/docker.mdx` 159, 182, 273;
  `guides/production/reverse-proxy.mdx` 55; `guides/production/run-as-a-service.mdx` 15, 42, 116;
  `guides/simple-programs/form-handler.mdx` 18; `guides/simple-programs/json-api.mdx` 19;
  `guides/tour/10-web-app.mdx` 15; `in-depth/compare.mdx` 293; `install/hello-world.mdx` 16, 37;
  `install/index.mdx` 55, 117, 124, 137, 142, 157, 162; `install/server.mdx` 30, 132, 151, 174, 307,
  333; `licenses.mdx` 27; `syntax/start-here.mdx` 15; `tools/api-document/index.mdx` 41;
  `tools/cli/index.mdx` 12, 81, 93, 108; `tools/editor/index.mdx` 150, 160;
  `tools/formatter/index.mdx` 24, 30; `tools/test-runner/index.mdx` 61.
- `bat`: `install/server.mdx` 23, 267, 274, 281, 288. `powershell`: `install/index.mdx` 132.
- `txt`: test runner output at `guides/example-apps/chat.mdx` 70, `notes.mdx` 56, `paperwork.mdx` 88,
  `poll.mdx` 65, `guides/extensions/rust.mdx` 213, `guides/tour/08-tests.mdx` 16 and
  `guides/why-novis/tests-in-code.mdx` 16. File trees at `guides/example-apps/chat.mdx` 107,
  `notes.mdx` 80, `paperwork.mdx` 117, `poll.mdx` 103 and `guides/how-to-use/project-layout.mdx` 9,
  54. A diagnostic at `guides/tour/01-methods-and-types.mdx` 20 and
  `guides/why-novis/no-null-surprises.mdx` 18. Program output at `install/hello-world.mdx` 41,
  `guides/example-apps/poll.mdx` 183 (an event stream) and `install/server.mdx` 50, 61, 143. Path
  lists at `install/server.mdx` 111, 119. LCOV at `tools/test-runner/coverage.mdx` 26.

**Stage 4: `nginx` (2), `caddyfile` (2), `dockerfile` (1), `rust` (1), `md` (1) and the six deploy
files.** `tools/nv/cmd/pdf-validate.ts:1`
- `guides/production/reverse-proxy.mdx` nginx 121, 431 and caddyfile 281, 460;
  `guides/production/docker.mdx` dockerfile 137; `guides/extensions/rust.mdx` rust 140 and md 263 (a
  four-backtick fence around an `html` one).
- `apps/chat/deploy/` and `apps/poll/deploy/`: each has `Caddyfile`, `nginx.conf` and
  `production.toml`, which the `apps` part checks (decision 4).

### Backlog

- From `realtime-example-apps`: a connection's `cpu_time` counts from the upgrade for its whole life,
  so poll's production `cpu_time = "5s"` may end a busy event stream after hours. Check whether the
  watchdog charges an unwatched connection isolate (`crates/nvs-host/src/watchdog.rs:1348`) and
  whether a connection wants a per-message budget (`rule:concurrency/a-connection-is-a-root-isolate`).
- From `realtime-example-apps`: whether `connect-src 'self'` allows `wss:` to the same host in every
  supported browser. The Chromium test covers `ws:` over `http:` only (`apps/chat/nvs.toml:14`).
- From `realtime-example-apps`: a screenshot of each app on its page, once `bun nv site --shoot` has
  an app shot (`tools/nv/cmd/site.ts:200`).
