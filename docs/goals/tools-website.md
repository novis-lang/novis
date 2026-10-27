---
looper: 1
title: The website shows every developer tool in detail, with pictures made from real runs
summary: "The website has a Tools area with an overview and one page per tool, every picture on those pages is generated from a real run and checked, the pages about running a tool live there and nowhere else, the home page points at it, every designed rule this goal owns is shipped, and the checks below pass."
context: [tools/nv/cmd/site.ts, website/astro.config.ts, website/config/site.mjs]
stages:
  - id: 1
    title: a terminal block shows what `nvs` printed
    status: done
    context: [tools/nv/cmd/site.ts, tools/nv/lib/term.ts, website/src/components/Terminal.astro, website/src/components/Snippet.astro, docs/rules/testing/a-terminal-block-shows-what-nvs-printed.md]
    extra:
      context:
        rules: [testing/a-terminal-block-shows-what-nvs-printed, testing/website-pages-are-stamped]
  - id: 7
    title: a replay renames, applies an action, shows a lens and walks calls
    status: done
    context: [tools/nv/cmd/site.ts, tools/nv/lib/replay.ts, website/src/components/Replay.astro, website/config/replay-frames.mjs, website/config/replay-player.mjs, tools/nv/test/site-replay.test.ts, docs/rules/testing/a-replay-renames-applies-an-action-and-shows-a-lens.md]
    extra:
      context:
        rules: [testing/a-replay-renames-applies-an-action-and-shows-a-lens, testing/a-replay-plays-what-the-language-server-returned]
  - id: 2
    title: a debugger replay plays what `nvs dap` returned
    status: done
    context: [tools/nv/cmd/site.ts, tools/nv/lib/dap-replay.ts, website/config/replay-player.mjs, docs/rules/testing/a-debugger-replay-plays-what-nvs-dap-returned.md]
    extra:
      context:
        rules: [testing/a-debugger-replay-plays-what-nvs-dap-returned, testing/a-replay-plays-what-the-language-server-returned]
  - id: 8
    title: a screenshot is taken from a real run
    status: done
    context: [tools/nv/cmd/site.ts, tools/nv/lib/shoot.ts, tools/nv/test/chromium.ts, package.json, docs/rules/testing/a-screenshot-is-taken-from-a-real-run.md]
    extra:
      context:
        rules: [testing/a-screenshot-is-taken-from-a-real-run, testing/the-viewer-shows-the-stream-and-hands-off-every-artifact]
  - id: 3
    title: the Tools area, its overview, and the three pages that move into it
    status: done
    context: [website/config/site.mjs, website/astro.config.ts, tools/nv/cmd/site.ts, website/src/components/Replay.astro, website/src/data/questions.json, website/src/content/docs/install/editor.mdx, website/src/content/docs/guides/testing/run-tests.mdx, website/src/content/docs/guides/testing/coverage.mdx, docs/rules/testing/the-tools-area-has-a-page-for-each-tool.md]
    extra:
      context:
        rules: [testing/the-tools-area-has-a-page-for-each-tool, testing/website-pages-are-stamped, testing/a-replay-plays-what-the-language-server-returned]
  - id: 4
    title: the editor, formatter, test runner and CLI pages
    status: done
    context: [website/astro.config.ts, website/src/data/questions.json, docs/examples/tools/cli, docs/examples/tools/editor]
    extra:
      context:
        rules: [testing/the-tools-area-has-a-page-for-each-tool, testing/website-pages-are-stamped, testing/examples-live-in-the-repository]
  - id: 5
    title: the debugger and API document pages
    status: done
    context: [website/astro.config.ts, website/src/data/questions.json, docs/examples/tools/cli]
    extra:
      context:
        rules: [testing/the-tools-area-has-a-page-for-each-tool, testing/a-debugger-replay-plays-what-nvs-dap-returned, testing/website-pages-are-stamped]
  - id: 9
    title: the debug viewer and profiler pages
    status: done
    context: [website/astro.config.ts, website/src/data/questions.json, tools/nv/lib/shoot.ts]
    extra:
      context:
        rules: [testing/the-tools-area-has-a-page-for-each-tool, testing/a-screenshot-is-taken-from-a-real-run, testing/website-pages-are-stamped]
  - id: 6
    title: the home page points at the Tools area, and every page is stamped
    status: open
    context: [website/src/components/home/Tooling.astro, website/src/content/docs/index.mdx, website/src/data/questions.json, website/site.lock.json]
    extra:
      context:
        rules: [testing/the-tools-area-has-a-page-for-each-tool, testing/website-pages-are-stamped]
  - id: 10
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
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
was, so every picture here is made from a real run and checked as a snippet's output is. Each page
waits for the goals whose tool it shows; the picture kinds and the area do not.

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
`example-apps/`. The new pages are `tools/index.mdx`, and `tools/<tool>/index.mdx` for `editor`,
`formatter`, `test-runner`, `cli`, `debugger`, `debug-viewer`, `profiler` and `api-document`.

**Which goals build what each page shows.** Debug viewer: `debug-service`, `debug-viewer`, and
`request-aware-editor`'s `nvs debug to-test`. Debugger: `debugger-core` and `debugger-adapter`.
Profiler: `trace-timeline`. Editor: the extension already built, `editor-refactors`, `editor-fixes`,
`request-aware-editor` and `api-document`'s `#[Api]` completion. Formatter: `formatter-complete` over
the `nvs fmt` already built. Test runner: the runner already built, `test-surface`, `mutation-testing`,
`bench-counters` and `diagnostic-records`' reports. API document: `api-document` over `nvs build
--openapi` and `nvs api diff`. CLI: `cli-housekeeping`, and `diagnostic-records`' `nvs check --json`.

**Gates.** A goal has landed when its file under `docs/goals/` is gone. Stages 1 and 3 need no other
goal. Stage 7's `action`, `lens` and hover card need none; its `rename` step needs `renameProvider` in
`crates/nvs-lsp/tests/handshake.rs` (`editor-refactors` stage 1), and its `calls` step needs
`callHierarchyProvider` there (`request-aware-editor` stage 2). Stage 2 needs `nvs dap` to step:
stages 1 and 2 of `docs/goals/debugger-adapter.md` are `done`, or that file is gone. Stage 8 needs
`debug-viewer` landed for the viewer's shots and stage 5 of `docs/goals/trace-timeline.md` for the
profile shot. Each page item in stages 4, 5 and 9 needs every goal the list above names for its tool.
A session whose gate fails takes the next item or stage that has none, and says so in the handoff.

**Lanes.** Stage 1 (`tools/nv/lib/term.ts`, `website/src/components/Terminal.astro`) and stage 7
(`tools/nv/lib/replay.ts`, `website/config/replay-*.mjs`) touch disjoint files beside
`tools/nv/cmd/site.ts`, where each adds its own `--check snippets` branch, so they run in order or as a
side goal that rebases on that one file. Stage 3 touches the area and the moved pages, disjoint from 1
and 7, and can run first. Stage 2 follows 7, because both extend `website/config/replay-player.mjs`; stage 8
adds `--shoot` and the `shots` part to `site.ts` and runs after 1 and 2 there. Stages 4, 5 and 9
share `website/astro.config.ts` and `website/src/data/questions.json` and run in order; each page
inside them is its own file. Stage 6 runs last.

**Narrow checks.** `bun nv site --check` takes parts (`tools/nv/cmd/site.ts:193`). Stages 1, 7 and 2
run `bun nv site --check snippets`; stage 8 the new `shots` part; stage 3 `structure questions guides`;
stages 4, 5 and 9 `tools snippets stale questions prose`. Only stage 6 runs the whole check. The
replay's browser test runs alone: `NV_BROWSER=1 bun test tools/nv/test/replay-browser.test.ts`.

**Trade-offs, all together.** Request path and binary: nothing. The site: one small script per kind
of picture, and the PNG files of the screenshots. CI: `--check` starts `nvs dap` once per debugger
replay and a browser once per screenshot, in the job that already builds `nvs`. Usability: a reader
sees each tool before installing it, and the picture is what the tool does today. Simplicity: one
area, three new kinds of checked picture, and no hand-drawn image to keep up to date.

## Stages

### 1. a terminal block shows what `nvs` printed

**Does:** a checked picture of the text a tool prints.

- [x] A `.term` beside a fixture lists commands, `--bless` writes `.term.out` from what `nvs` printed, and `--check snippets` fails when it changed (`rule:testing/a-terminal-block-shows-what-nvs-printed`). `tools/nv/lib/term.ts:1`, `tools/nv/cmd/site.ts:1211`, `tools/nv/cmd/site.ts:1287`
- [x] A `<Terminal>` component shows the block, with its masked values shown as written. `website/src/components/Terminal.astro:1`
- [x] Tests over a fixture for a fresh and a stale `.term.out`. `tools/nv/test/site-term.test.ts:1`
- [x] `rule:testing/a-terminal-block-shows-what-nvs-printed` `shipped`. `data/rules/testing/a-terminal-block-shows-what-nvs-printed.json:3`

### 7. a replay renames, applies an action, shows a lens and walks calls

**Does:** the editor replay covers the features beyond completion.

- [x] The replay gains `action` and `lens` steps and a hover card, recorded and checked like a list (`rule:testing/a-replay-renames-applies-an-action-and-shows-a-lens`). `tools/nv/lib/replay.ts:16`, `tools/nv/lib/replay.ts:223`, `website/config/replay-frames.mjs:29`, `tools/nv/cmd/site.ts:1129`
- [x] The `rename` step, once its gate holds. `tools/nv/lib/replay.ts:16`
- [x] The `calls` step, once its gate holds. `tools/nv/lib/replay.ts:16`
- [x] The player draws them. `website/config/replay-player.mjs:140`, `website/src/components/Replay.astro:63`
- [x] Tests over a fixture for each new step, and the browser test. `tools/nv/test/site-replay.test.ts:1`, `tools/nv/test/replay-browser.test.ts:32`
- [x] The declined bullet on hover cards in `rule:testing/a-replay-plays-what-the-language-server-returned` rewritten, and `rule:testing/a-replay-renames-applies-an-action-and-shows-a-lens` `shipped`. `docs/rules/testing/a-replay-plays-what-the-language-server-returned.md:56`, `data/rules/testing/a-replay-renames-applies-an-action-and-shows-a-lens.json:3`

### 2. a debugger replay plays what `nvs dap` returned

**Does:** the debugger's picture is a recorded `nvs dap` session.

- [x] A `.dap` beside a snippet lists breakpoints and steps, `--bless` records the answers into `.dap.out`, and `--check snippets` fails when `nvs dap` returns something else (`rule:testing/a-debugger-replay-plays-what-nvs-dap-returned`). `tools/nv/lib/dap-replay.ts:1`, `tools/nv/cmd/site.ts:1172`, `tools/nv/cmd/site.ts:1208`
- [x] The player shows the breakpoint, the stopped line, the call stack and the variables. `website/config/replay-player.mjs:56`
- [x] Tests over a fixture for a fresh and a stale `.dap.out`. `tools/nv/test/site-dap.test.ts:1`
- [x] `rule:testing/a-debugger-replay-plays-what-nvs-dap-returned` `shipped`. `data/rules/testing/a-debugger-replay-plays-what-nvs-dap-returned.json:3`

### 8. a screenshot is taken from a real run

**Does:** the viewer's and the profiler's pictures are screenshots of real runs, checked through text.

- [x] `bun nv site --shoot` serves an app under `apps/` in development with `[debug] store` on, sends the fixed requests, starts `nvs debug`, and takes each `.shot`'s screenshot with the Chromium driver beside its `.shot.out` (`rule:testing/a-screenshot-is-taken-from-a-real-run`). `tools/nv/lib/shoot.ts:1`, `tools/nv/cmd/site.ts:1739`, `tools/nv/cmd/site.ts:1567`, `tools/nv/test/chromium.ts:31`
- [x] A profile shot opens a real `--profile=speedscope` export in the speedscope build the extension bundles, and its `.shot.out` holds the frames and call counts without times. `tools/nv/lib/shoot.ts:1`
- [x] A `shots` part of `--check` fails a stale or missing `.shot.out`, and tests for it. `tools/nv/cmd/site.ts:193`, `tools/nv/cmd/site.ts:1646`, `tools/nv/test/site-shots.test.ts:1`
- [x] `rule:testing/a-screenshot-is-taken-from-a-real-run` `shipped`. `data/rules/testing/a-screenshot-is-taken-from-a-real-run.json:3`

### 3. the Tools area, its overview, and the three pages that move into it

**Does:** Tools is an area with its sidebar, the three pages and their snippets move with no trace,
and the overview page exists.

- [x] Tools joins `AREAS` after Reference, the sidebar gains its group before In-Depth, and the `structure` check names five areas (`rule:testing/the-tools-area-has-a-page-for-each-tool`). `website/config/site.mjs:42`, `website/astro.config.ts:669`, `tools/nv/cmd/site.ts:151`, `tools/nv/cmd/site.ts:612`
- [x] The three pages and their snippets move, with their entries in `questions.json` keyed to the new addresses and their sidebar entries gone. `website/astro.config.ts:447`, `website/astro.config.ts:462`, `website/astro.config.ts:702`, `website/src/data/questions.json:416`, `website/src/data/questions.json:427`, `website/src/data/questions.json:930`, `website/src/content/docs/guides/testing/run-tests.mdx:12`
- [x] The two entries for the moved pages in the testing guide's list are removed, not repointed, so the list names only pages that are still in Guides. `website/src/content/docs/guides/testing/index.mdx:10`, `website/src/content/docs/guides/testing/index.mdx:16`
- [x] Every other link to an old address points at the new one: the install page, the project layout page, the moved coverage page's own link, the replay component and its browser test. `website/src/content/docs/install/index.mdx:124`, `website/src/content/docs/guides/how-to-use/project-layout.mdx:48`, `website/src/content/docs/guides/testing/coverage.mdx:26`, `website/src/components/Replay.astro:63`, `tools/nv/test/replay-browser.test.ts:32`
- [x] The replay rule's notice sentence names the editor page instead of the install page. `docs/rules/testing/a-replay-plays-what-the-language-server-returned.md:19`
- [x] The overview page with a line per tool. `website/src/content/docs/tools/index.mdx:1`
- [x] A `tools` part of `--check`: each of the eight tool pages exists, shows a checked picture and covers its features. It names each page still owed until stages 4, 5 and 9 land. `tools/nv/cmd/site.ts:193`, `tools/nv/cmd/site.ts:1646`

### 4. the editor, formatter, test runner and CLI pages

**Does:** four tool pages, each with its pictures and its `covers:` list.

**Split:** one writing subagent per page, one after another, each with the page's gate checked first,
its examples and pictures, and AGENTS.md § *Text an end user reads*. The session alone edits
`website/astro.config.ts`'s sidebar and `website/src/data/questions.json`, runs `--bless`, and checks
each page.

- [x] Editor: the moved page, then rename, the code actions and generators, the lenses with "used by N routes, M tests", call hierarchy, the route hover and the taint path, each as a replay. Examples from `docs/examples/tools/editor/`. `website/src/content/docs/tools/editor/index.mdx:1`
- [x] Formatter: what `nvs fmt` changes, as a terminal block of `--diff`, and `--check` with its exit codes. `website/src/content/docs/tools/formatter/index.mdx:1`
- [x] Test runner: the moved pages, then the options, `#[Property]`, mutation testing and benches, each with a terminal block. `website/src/content/docs/tools/test-runner/index.mdx:1`, `website/src/content/docs/tools/test-runner/coverage.mdx:1`, `docs/examples/tools/cli/nvs-test/about.md:1`
- [x] CLI: `nvs check --json`, the cache commands, `nvs update-check` and `nvs telemetry`, each with a terminal block, and a link to the full command list. `website/src/content/docs/tools/cli/index.mdx:1`, `website/src/content/docs/reference/index.mdx:13`
- [x] Each page's sidebar entry, its questions, and its stamp (`rule:testing/website-pages-are-stamped`). `website/astro.config.ts:669`, `website/src/data/questions.json:1`

### 5. the debugger and API document pages

**Does:** the two pages whose pictures are debugger replays and terminal blocks.

**Split:** one writing subagent per page, one after another, as in stage 4. The session alone edits the
sidebar and `questions.json` and runs `--bless`.

- [x] Debugger: `launch.json`, a breakpoint, stepping and the variables, as debugger replays. `website/src/content/docs/tools/debugger/index.mdx:1`
- [x] API document: `nvs build --openapi` and `nvs api diff` as terminal blocks, and `#[Api]` completion as a replay. `website/src/content/docs/tools/api-document/index.mdx:1`, `docs/examples/tools/cli/nvs-build-openapi/about.md:1`, `docs/examples/tools/cli/nvs-api-diff/about.md:1`
- [x] Each page's sidebar entry, its questions, and its stamp. `website/astro.config.ts:669`, `website/src/data/questions.json:1`

### 9. the debug viewer and profiler pages

**Does:** the two pages whose pictures are screenshots.

**Split:** one writing subagent per page, one after another, after the session has run `--shoot`. The
session alone edits the sidebar and `questions.json`.

- [x] Debug viewer: starting it, the request list, a request's timeline, search, grouped exceptions, compare with two responses' diff, replay, and a request made into a test that asserts its response, as screenshots and a terminal block. `website/src/content/docs/tools/debug-viewer/index.mdx:1`
- [x] Profiler: `--profile=callgrind` and `--profile=speedscope` as terminal blocks, the speedscope screenshot, and the sentence that the probe per call makes call-heavy code look slower. `website/src/content/docs/tools/profiler/index.mdx:1`
- [x] Each page's sidebar entry, its questions, and its stamp. `website/astro.config.ts:669`, `website/src/data/questions.json:1`
- [x] A `<Shot src="tools/debug-viewer/requests.shot"/>` component shows the `.png` beside a `.shot`, and `shotShape` names a `.shot` no page shows, the way `snippetShape` names a `.term`. `website/src/components/Terminal.astro:1`, `tools/nv/cmd/site.ts:1483`

### 6. the home page points at the Tools area, and every page is stamped

**Does:** the home page's Tooling section shows the tools and links to Tools, and the whole site
passes its check.

- [ ] The Tooling section lists the tools with one line each and links to the overview, and no link points at a moved page. `website/src/components/home/Tooling.astro:29`, `website/src/components/home/Tooling.astro:76`
- [ ] The overview shows each tool's first picture. `website/src/content/docs/tools/index.mdx:1`
- [ ] `bun nv site --check` passes, `rule:testing/the-tools-area-has-a-page-for-each-tool` is `shipped`, and `bun nv owners --closes tools-website` passes. `data/rules/testing/the-tools-area-has-a-page-for-each-tool.json:3`

### 10. the goal is verified once, and every failure is fixed

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

Stages 1, 7, 2, 8, 3, 4, 5 and 9 are done. Tools is the fourth of five areas
(`website/config/site.mjs:42`), and its sidebar (`website/astro.config.ts:679`) and the overview's list
(`website/src/content/docs/tools/index.mdx:10`) name all eight tool pages in `TOOL_SECTIONS`' order
(`tools/nv/cmd/site.ts:850`).

- **`<Shot src alt caption>`**, `website/src/components/Shot.astro`, shows the `.png` beside a `.shot`.
  `shotShape` (`tools/nv/cmd/site.ts:1520`) names a `.shot` no page shows and a `<Shot src>` that names
  no `.shot`. A viewer shot's `app .` serves the directory beside it, which is how `exceptions.shot`
  gets an error no gate catches: every app under `apps/` has a gate whose `failed()` catches it, and the
  exceptions page lists only an error nothing caught. `projectPaths` (`tools/nv/lib/shoot.ts:111`)
  writes a path in the run's copy from the project root, and a made test's class name has its 8 trace
  id digits masked, so a `.shot.out` is the same on every run and runner.
- **The debug viewer page**, `tools/debug-viewer/index.mdx`, covers `tools:cli/nvs-debug`,
  `tools:cli/the-debug-viewer` and `tools:cli/nvs-debug-to-test`, with seven shots (`requests`,
  `timeline`, `search`, `exceptions`, `compare`, `replay`, `to-test`) and one terminal block,
  `unknown-trace`. A `--help` block is not used: its `Usage:` line prints `nvs.exe` on Windows.
- **The profiler page**, `tools/profiler/index.mdx`, covers `tools:cli/profiling`, with the
  `callgrind` and `speedscope` terminal blocks (each names its file with `--profile-out`, since the
  default name has the process id) and the `fibonacci` shot.
- The sweep's reds were fixed on HEAD before this session: nightly run 40291801939's by `c244b4b36`,
  `e384a0ae8` and `0288e849d`; `env-secret-proofs` passes when rerun.

### Next

**Stage 6: the home page points at the Tools area, and every page is stamped.** No gate. The session
edits alone: one component and the overview.

- [ ] The Tooling section links each row to its tool page where one exists (`nvs fmt` has none, `nvs test` could go to `/tools/test-runner/`), its "more" link goes to `/tools/`, and no link points at a moved page. `website/src/components/home/Tooling.astro:18`, `website/src/components/home/Tooling.astro:76`
- [ ] The overview shows each tool's first picture beside its line, a `<Shot>`, `<Terminal>` or `<Replay>` each. `website/src/content/docs/tools/index.mdx:10`
- [ ] `bun nv site --check` passes (with `--nvs target/debug/nvs.exe` unless the release build is rebuilt), and the five header areas fit at mid widths in a browser. `tools/nv/cmd/site.ts:193`
- [ ] `rule:testing/the-tools-area-has-a-page-for-each-tool` is `shipped`, and `bun nv owners --closes tools-website` passes. `data/rules/testing/the-tools-area-has-a-page-for-each-tool.json:3`

Then stage 10, the gate. No test is expected to go red.

### Backlog

- The viewer shows a full file path in "Thrown at" and in a comparison's queries, so a shot's picture
  shows the run's temporary folder; only its text is masked. The viewer could show a path from the
  project root.
- A `.shot` has no `scroll` step, so `compare.png` shows the request rows and not the response rows
  below them. The page says the response is compared in words.
- A gate's `failed()` hides a route's error from the exceptions page. A developer who debugs a 500
  probably wants it listed; that is a `debug-viewer` call.
- `lib/term.ts` could write a relative path's `\` as `/` in a recording, and mask the binary's name in
  a `Usage:` line, so a `.term` may walk a directory or print `--help`.
- `nvs cache gc`, `nvs cache clear` and `[opcache] file_cache_max_size` have no section in
  `docs/reference/tools/10-cli.md`; `nvs cache --help` names them.
- A replay of the field names inside `#[Api(...)]` needs `nvs lsp` to complete in a half-written
  attribute with `'])` after the cursor; `tests/lsp/completion/an-api-attribute-offers-its-fields.lspt`
  covers only the clean case.
- A `.dap.out` could list an object's properties, so a debugger replay could show one.
