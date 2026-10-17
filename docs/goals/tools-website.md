---
looper: 1
title: The website shows every developer tool in detail, with pictures made from real runs
summary: "The website has a Tools area with an overview and one page per tool, every picture on those pages is generated from a real run and checked, the pages about running a tool live there and nowhere else, the home page points at it, every designed rule this goal owns is shipped, and the checks below pass."
context: [tools/nv/cmd/site.ts, website/astro.config.ts, website/config/site.mjs]
stages:
  - id: 1
    title: a terminal block shows what `nvs` printed, and a replay renames, applies an action and shows a lens
    status: open
    context: [tools/nv/cmd/site.ts, tools/nv/lib/replay.ts, website/src/components/Replay.astro, website/config/replay-frames.mjs, website/config/replay-player.mjs, tools/nv/test/site-replay.test.ts, docs/rules/testing/a-terminal-block-shows-what-nvs-printed.md, docs/rules/testing/a-replay-renames-applies-an-action-and-shows-a-lens.md]
    extra:
      context:
        rules: [testing/a-terminal-block-shows-what-nvs-printed, testing/a-replay-renames-applies-an-action-and-shows-a-lens, testing/a-replay-plays-what-the-language-server-returned, testing/website-pages-are-stamped]
  - id: 2
    title: a debugger replay plays what `nvs dap` returned, and a screenshot is taken from a real run
    status: open
    context: [tools/nv/cmd/site.ts, tools/nv/test/chromium.ts, website/config/replay-player.mjs, package.json, docs/rules/testing/a-debugger-replay-plays-what-nvs-dap-returned.md, docs/rules/testing/a-screenshot-is-taken-from-a-real-run.md]
    extra:
      context:
        rules: [testing/a-debugger-replay-plays-what-nvs-dap-returned, testing/a-screenshot-is-taken-from-a-real-run, testing/a-replay-plays-what-the-language-server-returned, testing/the-viewer-shows-the-stream-and-hands-off-every-artifact]
  - id: 3
    title: the Tools area, its overview, and the three pages that move into it
    status: open
    context: [website/config/site.mjs, website/astro.config.ts, tools/nv/cmd/site.ts, website/src/components/Replay.astro, website/src/data/questions.json, website/src/content/docs/install/editor.mdx, website/src/content/docs/guides/testing/run-tests.mdx, website/src/content/docs/guides/testing/coverage.mdx, docs/rules/testing/the-tools-area-has-a-page-for-each-tool.md]
    extra:
      context:
        rules: [testing/the-tools-area-has-a-page-for-each-tool, testing/website-pages-are-stamped, testing/a-replay-plays-what-the-language-server-returned]
  - id: 4
    title: the editor, formatter, test runner and CLI pages
    status: open
    context: [website/astro.config.ts, website/src/data/questions.json, docs/examples/tools/cli, docs/examples/tools/editor]
    extra:
      context:
        rules: [testing/the-tools-area-has-a-page-for-each-tool, testing/website-pages-are-stamped, testing/examples-live-in-the-repository]
  - id: 5
    title: the debugger, debug viewer, profiler and API document pages
    status: open
    context: [website/astro.config.ts, website/src/data/questions.json, docs/examples/tools/cli]
    extra:
      context:
        rules: [testing/the-tools-area-has-a-page-for-each-tool, testing/a-debugger-replay-plays-what-nvs-dap-returned, testing/a-screenshot-is-taken-from-a-real-run, testing/website-pages-are-stamped]
  - id: 6
    title: the home page points at the Tools area, and every page is stamped
    status: open
    context: [website/src/components/home/Tooling.astro, website/src/content/docs/index.mdx, website/src/data/questions.json, website/site.lock.json]
    extra:
      context:
        rules: [testing/the-tools-area-has-a-page-for-each-tool, testing/website-pages-are-stamped]
checks:
  - id: site-check
    stage: 6
    name: every part of the website's check passes
    run: [bun, nv, site, --check]
    reach: true
    floor: false
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, tools-website]
    reach: true
    floor: false
---

# The website shows every developer tool in detail, with pictures made from real runs

## Why

The developer-tooling goals build a debug viewer, a debugger, a profiler, a deep editor, a finished formatter, a test runner
with mutation testing and benches, a complete API document and the housekeeping commands. The website
shows almost none of it. The tools are spread across Guides and Install: running tests and coverage in
`guides/testing/`, the editor in `install/editor.mdx`, and the rest nowhere. The home page's Tooling
section links to the editor install page (`website/src/components/home/Tooling.astro:29`). The user's
words: "we should list each and every of this powerful features in detail, with examples and
screenshots, so devs instantly get the picture of it." A picture that nothing checks shows a tool as it
was, so every picture here is made from a real run and checked as a snippet's output is. It runs
after every goal whose tool it shows.

## Standing decisions

**A new top-level Tools area.** The user decided this. It has an overview page that shows every tool
with a picture and one line, and one page per tool: debug viewer, debugger, profiler, editor,
formatter, test runner, API document and CLI. The home page's Tooling section points at it.
`rule:testing/the-tools-area-has-a-page-for-each-tool` holds it.

**A page about a tool moves into Tools and leaves no trace.** The user decided this: "if guides move
to tools, we dont need to keep a trace from that in guides". No stub page, no "moved" note, no link
back and no redirect, and every link to the old address is changed. A page about running a tool moves,
and a page about writing Novis code stays.

**Every picture is generated and checked, never drawn by hand.** The user decided this. Viewer
screenshots come from Playwright over a real `nvs debug`. Editor features use the replay component,
extended to rename, code actions, code lenses and call hierarchy. The debugger gets a replay recorded from real `nvs
dap` answers. A check fails when a picture is out of date. Each extension is a new rule owned by this
goal.

**How each kind of picture is checked.** The planning session's call. A replay and a terminal block
are checked against what the binary returns now, as a `.replay.out` is. A screenshot is checked
through a text file beside it: the page's visible text with times, dates and trace ids masked, and
the hash of the viewer's assets. Pixels are never compared, because two machines draw the same page
differently. The profiler's picture is a screenshot of the speedscope build the VS Code extension
bundles (`trace-timeline`, a user decision) opening a real export, so the website draws no flame chart
and adds no dependency (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).

**Where Tools sits, and the shape of a tool page.** The planning session's call. Tools is the fourth
area, after Reference and before In-Depth. Each tool is a directory under `tools/`, so the test
runner can keep coverage as a page of its own. A CLI-output picture is a terminal block. The test for
a page is its subject: using one of the eight tools moves, and writing Novis code or running a program
in production stays.

**Page by page, under `website/src/content/docs/`.** Three pages move. `install/editor.mdx` becomes
`tools/editor/index.mdx`, `guides/testing/run-tests.mdx` becomes `tools/test-runner/index.mdx`, and
`guides/testing/coverage.mdx` becomes `tools/test-runner/coverage.mdx`. Their snippets move from
`website/snippets/install/property-key-*` and `website/snippets/guides/testing/run-tests/` to
`website/snippets/tools/`. Every other page stays: `guides/index.mdx`; the 12 pages of
`why-novis/`, which are about the language; the 11 of `tour/`, where `08-tests.mdx` writes tests;
the 7 of `simple-programs/` and the 11 of `cookbook/`, which are programs; the 8 of `how-to-use/`,
whose `configuration.mdx` and `project-layout.mdx` set up a program; `testing/index.mdx`,
`test-a-route.mdx` and `test-doubles.mdx`, which write tests; the 6 of `production/`, whose
`run-as-a-service.mdx` and `limits-and-logs.mdx` run a server in production; and the 3 of
`example-apps/`.

**Which goals build what each page shows.** Debug viewer: `debug-service`, `debug-viewer`, and
`request-aware-editor`'s `nvs debug to-test`. Debugger: `debugger-core` and `debugger-adapter`.
Profiler: `trace-timeline`. Editor: the extension already built, `editor-refactors`, `editor-fixes`,
`request-aware-editor` and `api-document`'s `#[Api]` completion. Formatter: `formatter-complete` over
the `nvs fmt` already built. Test runner: the runner already built, `test-surface`, `mutation-testing`,
`bench-counters` and `diagnostic-records`' reports. API document: `api-document` over `nvs build
--openapi` and `nvs api diff`. CLI: `cli-housekeeping`, and `diagnostic-records`' `nvs check --json`.

**Trade-offs, all together.** Request path and binary: nothing. The site: one small script per kind
of picture, and the PNG files of the screenshots. CI: `--check` starts `nvs dap` once per debugger
replay and a browser once per screenshot, in the job that already builds `nvs`. Usability: a reader
sees each tool before installing it, and the picture is what the tool does today. Simplicity: one
area, three new kinds of checked picture, and no hand-drawn image to keep up to date.

## Stages

### 1. a terminal block shows what `nvs` printed, and a replay renames, applies an action and shows a lens

**Does:** two new kinds of checked picture for text a tool prints and for editor features beyond
completion.

- [ ] A `.term` beside a fixture lists commands, `--bless` writes `.term.out` from what `nvs` printed, and `--check snippets` fails when it changed (`rule:testing/a-terminal-block-shows-what-nvs-printed`). `tools/nv/cmd/site.ts:1177`, `tools/nv/cmd/site.ts:189`
- [ ] A `<Terminal>` component shows the block, with its masked values shown as written.
- [ ] The replay gains `rename`, `action`, `lens` and `calls` steps and a hover card, recorded and checked like a list (`rule:testing/a-replay-renames-applies-an-action-and-shows-a-lens`). `tools/nv/cmd/site.ts:70-112`, `tools/nv/lib/replay.ts`
- [ ] The player draws them. `website/config/replay-player.mjs`, `website/src/components/Replay.astro`
- [ ] Tests over a fixture for each new step and for a stale `.term.out`, and the declined bullet in `rule:testing/a-replay-plays-what-the-language-server-returned` rewritten. Both rules `shipped`. `tools/nv/test/site-replay.test.ts`, `tools/nv/test/replay-browser.test.ts`

### 2. a debugger replay plays what `nvs dap` returned, and a screenshot is taken from a real run

**Does:** the debugger's picture is a recorded `nvs dap` session, and the viewer's and the profiler's
are screenshots of real runs.

- [ ] A `.dap` beside a snippet lists breakpoints and steps, `--bless` records the answers into `.dap.out`, and `--check snippets` fails when `nvs dap` returns something else (`rule:testing/a-debugger-replay-plays-what-nvs-dap-returned`).
- [ ] The player shows the breakpoint, the stopped line, the call stack and the variables.
- [ ] `bun nv site --shoot` serves an app under `apps/` in development with `[debug] store` on, sends the fixed requests, starts `nvs debug`, and takes each `.shot`'s screenshot with Playwright beside its `.shot.out` (`rule:testing/a-screenshot-is-taken-from-a-real-run`). `tools/nv/cmd/site.ts:1432-1448`, `tools/nv/test/chromium.ts`
- [ ] A profile shot opens a real `--profile=speedscope` export in the speedscope build the extension bundles, and its `.shot.out` holds the frames and call counts without times.
- [ ] A `shots` part of `--check` fails a stale or missing `.shot.out`, and tests for it. Both rules `shipped`. `tools/nv/cmd/site.ts:189`

### 3. the Tools area, its overview, and the three pages that move into it

**Does:** Tools is an area with its sidebar, the three pages and their snippets move with no trace,
and the overview page exists.

- [ ] Tools joins `AREAS` after Reference, the sidebar gains its group, and the `structure` check names five areas (`rule:testing/the-tools-area-has-a-page-for-each-tool`). `website/config/site.mjs:37-47`, `website/astro.config.ts:160`, `tools/nv/cmd/site.ts:147`, `tools/nv/cmd/site.ts:603-630`
- [ ] The three pages and their snippets move, with their entries in `questions.json` keyed to the new addresses and their sidebar entries gone. `website/astro.config.ts:447`, `website/astro.config.ts:462`, `website/astro.config.ts:700`, `website/src/data/questions.json:405`, `website/src/data/questions.json:416`, `website/src/data/questions.json:892`
- [ ] The two entries for the moved pages in the testing guide's list are removed, not repointed, so the list names only pages that are still in Guides. `website/src/content/docs/guides/testing/index.mdx:10`, `website/src/content/docs/guides/testing/index.mdx:16`
- [ ] Every other link to an old address points at the new one. `website/src/content/docs/install/index.mdx:108`, `website/src/content/docs/guides/how-to-use/project-layout.mdx:48`, `website/src/components/Replay.astro:63`, `tools/nv/test/replay-browser.test.ts:32`
- [ ] The replay rule's notice sentence names the editor page instead of the install page. `docs/rules/testing/a-replay-plays-what-the-language-server-returned.md`
- [ ] A `tools` part of `--check`: each of the eight tool pages exists, shows a checked picture and covers its features. The overview page with a line per tool.

### 4. the editor, formatter, test runner and CLI pages

**Does:** four tool pages, each with its pictures and its `covers:` list.

- [ ] Editor: the moved page, then rename, the code actions and generators, the lenses with "used by N routes, M tests", call hierarchy, the route hover and the taint path, each as a replay. Examples from `docs/examples/tools/editor/`.
- [ ] Formatter: what `nvs fmt` changes, as a terminal block of `--diff`, and `--check` with its exit codes.
- [ ] Test runner: the moved pages, then the options, `#[Property]`, mutation testing and benches, each with a terminal block. `docs/examples/tools/cli/nvs-test`
- [ ] CLI: `nvs check --json`, the cache commands, `nvs update-check` and `nvs telemetry`, each with a terminal block, and a link to the full command list. `website/src/content/docs/reference/index.mdx:13`
- [ ] Each page in the voice of AGENTS.md § *Text an end user reads*, with its questions, and stamped.

### 5. the debugger, debug viewer, profiler and API document pages

**Does:** the four pages whose pictures are a debugger replay, screenshots and a terminal block.

- [ ] Debugger: `launch.json`, a breakpoint, stepping and the variables, as debugger replays.
- [ ] Debug viewer: starting it, the request list, a request's timeline, search, grouped exceptions, compare with two responses' diff, replay, and a request made into a test that asserts its response, as screenshots and a terminal block.
- [ ] Profiler: `--profile=callgrind` and `--profile=speedscope` as terminal blocks, the speedscope screenshot, and the sentence that the probe per call makes call-heavy code look slower.
- [ ] API document: `nvs build --openapi` and `nvs api diff` as terminal blocks, and `#[Api]` completion as a replay. `docs/examples/tools/cli/nvs-build-openapi`, `docs/examples/tools/cli/nvs-api-diff`
- [ ] Each page in the plain voice, with its questions, and stamped.

### 6. the home page points at the Tools area, and every page is stamped

**Does:** the home page's Tooling section shows the tools and links to Tools, and the whole site
passes its check.

- [ ] The Tooling section lists the tools with one line each and links to the overview, and no link points at a moved page. `website/src/components/home/Tooling.astro:29`, `website/src/components/home/Tooling.astro:76`
- [ ] The overview shows each tool's first picture.
- [ ] `bun nv site --check` passes, every rule this goal owns is `shipped`, and `bun nv owners --closes tools-website` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written. It runs after every other developer-tooling goal, because each page shows a tool another goal builds.

### Next

Stage 1, terminal blocks and the replay's new steps, on `tools/nv/cmd/site.ts` and
`tools/nv/lib/replay.ts`:
- the `.term` format and its check (`rule:testing/a-terminal-block-shows-what-nvs-printed`). `tools/nv/cmd/site.ts:1177`
- the `rename`, `action`, `lens` and `calls` steps and the hover card. `tools/nv/cmd/site.ts:70`
- the player and its browser test. `website/config/replay-player.mjs`

### Backlog
