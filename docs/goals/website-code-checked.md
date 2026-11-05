---
looper: 1
title: Every block of code the website shows is checked by a real run
summary: "Reached when bun nv site --check code fails on any code block of a hand-written page or an example app that no check runs and that no marker exempts with a reason, and is green."
context: [tools/nv/cmd/site.ts, tools/nv/cmd/reference.ts, website/README.md, .github/workflows/ci.yml]
stages:
  - id: 1
    title: "every code block of the website is listed with what checks it, and the code part exists"
    status: open
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
   failure. Stage 5 writes this as `rule:testing/every-code-block-on-the-website-is-checked`.
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

## Stages

### 1. every code block of the website is listed with what checks it, and the code part exists

**Does:** turns the counts in *Why* into a list, and lands the part that later stages fill.

**Split:** a `reader` subagent lists every fenced block of every hand-written page and every file of
`apps/*/deploy/`, as `page:line language meta`, with the check that runs it today or `none`.

- [ ] The list, written into this goal's `### Next` as the work of stages 2 to 4.
  `website/src/content/docs/guides/production/reverse-proxy.mdx:1`
- [ ] `site --check code` exists, reads each fence's language and meta, fails on a `skip` with no
  reason, and names every block no check runs. It is not yet in CI. `tools/nv/cmd/site.ts:318`
- [ ] A Bun test for the part over a fixture page with each kind of fence. `tools/nv/test/site.test.ts:1`

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
- [ ] `rule:testing/every-code-block-on-the-website-is-checked`, rendered, and named in
  `website/README.md`'s table of who owns which file. `docs/rules/testing/in-process-request.md:1`

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written by the interactive session of 2026-11-05. Nothing of it is built. The `apps` part already
runs in CI and in `bun nv ci --check` (`b2864a13e1`), so every example app's tests and stored files
are checked; this goal covers the rest of what the website shows.

### Next

**Stage 1**, over `tools/nv/cmd/site.ts` and the hand-written pages; no gate.

- [ ] The list of every block. `website/src/content/docs/guides/production/reverse-proxy.mdx:1`
- [ ] The `code` part, not in CI yet. `tools/nv/cmd/site.ts:318`
- [ ] Its Bun test. `tools/nv/test/site.test.ts:1`

### Backlog

- The `Core` reference cards' prose examples outside `docs/examples/`, if any exist, are listed by
  stage 1 too (`tools/nv/renderers/website-core.ts:1`).
