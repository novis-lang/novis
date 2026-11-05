---
looper: 1
title: The chat and poll apps are production-ready starting points for WebSockets and event streams
summary: "Reached when apps/chat and apps/poll pass bun nv site --check apps, a browser test drives each app's own script in Chromium, and a guide page tells a reader when to use a WebSocket and when an event stream."
context: [apps/chat/README.md, apps/poll/README.md, website/src/content/docs/guides/example-apps/chat.mdx, website/src/content/docs/guides/example-apps/poll.mdx, website/src/lib/apps.ts]
stages:
  - id: 1
    title: "both apps land with one production setup, and the website's app check is green over them"
    status: open
    context: [tools/nv/cmd/site.ts, website/src/content/docs/guides/production/reverse-proxy.mdx]
  - id: 2
    title: "a browser test drives chat.js and poll.js in Chromium against the served apps"
    status: open
    context: [tools/nv/test/registry-browser.test.ts, tools/nv/test/chromium.ts, tools/nv/cmd/ci-changes.ts]
  - id: 3
    title: "a guide page says when to use a WebSocket and when an event stream, and links both apps"
    status: open
    context: [website/src/content/docs/guides/how-to-use/index.mdx, website/astro.config.ts]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: apps-check
    stage: 1
    name: every app under apps/ passes nvs test, serves / and matches its copies
    run: [bun, nv, site, --check, apps]
    timeout: 1800
  - id: realtime-browser
    stage: 2
    name: chat.js and poll.js work in Chromium against the served apps
    run: [bun, test, tools/nv/test/realtime-browser.test.ts]
    timeout: 900
---

# The chat and poll apps are production-ready starting points for WebSockets and event streams

## Why

Novis has WebSockets (`Core\Socket`), event streams (`Core\Sse`) and topics (`Core\Topic`) in the
server itself, so an app needs no second push server and no client library. Until 2026-11-05 the only
place a reader met them was the three-example reference cards, which show one call each and never a
whole app. The user asked for real apps "with the full setup (proxy, novis, how to use it)" that a
reader can copy as the start of their own, the same way `apps/notes` and `apps/paperwork` are used,
and then for them to be production-ready.

The interactive session of 2026-11-05 wrote both apps and their pages, and fixed three bugs on the
way, all committed separately:

- a `#[Test(server: true)]` server ran its program without the tree's `[capabilities]`
  (`rule:testing/in-process-request`),
- `Core\Test::request` answered `Core\Response::sendFile` with an empty `text/html` body,
- the reverse proxy guide said a Unix socket needs no `trusted_proxies`, which leaves every browser
  form and WebSocket failing the `Origin` check with `403`.

What is left is to land both apps on one production shape, to prove the browser scripts in a real
browser (nobody has run `chat.js` or `poll.js` in one yet), and to give the reader the page that
says which of the two features to pick.

## Standing decisions

1. **Ports:** chat `127.0.0.1:8144`, poll `127.0.0.1:8145`. `tools/nv/test/app.test.ts` fails on two
   apps sharing a port.
2. **The browser code is a file under `public/`,** sent by a route with `Core\Response::sendFile` so
   it works in production mode, where `nvs serve` sends no static file. No JavaScript library, and
   the scripts write the page only with `textContent` and `append`.
3. **A strict `Content-Security-Policy`** through `[http.headers] content_security_policy` in each
   app's `nvs.toml`: `default-src 'none'`, scripts, styles and connections from `'self'` only. No
   inline script or style is added to either app.
4. **Identity:** chat keeps the visitor's name in `Core\Session`, read by the upgrade request, and
   passes only the name and the room to the connection through `args:`. Poll gives each browser a
   long-lived voter id, and a second vote from the same browser moves that vote. A login is the
   reader's to add, and each page says so.
5. **Topic names are built from checked values only** (a room from the app's own list, a poll id from
   a `uint` route parameter), and the published value carries no secret.
6. **`deploy/` is one shape in both apps:** `deploy/Caddyfile`, `deploy/nginx.conf` and one
   production configuration file the operator passes. Stage 1 makes the two apps agree on it; the
   shape the two writers chose independently is reconciled to the simpler one, and the page's
   `## Deploy it` walks through exactly those files.
7. **Topics stay in one process.** Neither app adds a broker. Each page says that several machines
   need a message queue in application code (`rule:core-classes/topic`).
8. **Tests:** unit tests use the in-memory `scratch` database; a live test (`#[Test(server: true)]`)
   uses the app's own database and deletes the rows it wrote.
9. **The browser test** (stage 2) is a new `tools/nv/test/realtime-browser.test.ts` on the shared
   driver `tools/nv/test/chromium.ts`, run like `registry-browser.test.ts`: a copy of each app under
   `.agent-tmp/`, served by `nvs serve` on a free port, `NVS_BIN` honoured, skipped without Chromium
   unless `NV_BROWSER` is set. It is added to CI's `browser` job and to the `browser` list in
   `tools/nv/cmd/ci-changes.ts`, with `apps/chat/` and `apps/poll/`.
10. **Lanes:** stage 1 first. Stages 2 and 3 touch disjoint files and may run as side goals once
    stage 1 is committed.

## Stages

### 1. both apps land with one production setup, and the website's app check is green over them

**Does:** reconciles the two `deploy/` folders, runs the website's checks over both apps, and commits
the apps with their pages.

- [ ] Make `apps/chat/deploy/` and `apps/poll/deploy/` the same shape (decision 6), and each page's
  `## Deploy it` match its files. `apps/chat/deploy/Caddyfile:1`, `apps/poll/deploy/Caddyfile:1`
- [ ] `bun nv site --check apps --nvs target/release/nvs.exe` is green; it runs `nvs test .` and
  serves `/` for every app. `tools/nv/cmd/site.ts:1913`
- [ ] `bun nv site --check structure guides prose stale` and `bun nv proofs --comments` over every
  `.nvs`, `.mdx`, `.toml`, `.conf` and `Caddyfile` of both apps are green.
  `website/src/content/docs/guides/example-apps/chat.mdx:1`
- [ ] Each page's `## Run the tests` block is the real output of `nvs test .`.
  `website/src/content/docs/guides/example-apps/poll.mdx:66`
- [ ] Commit the apps, the two pages, the index, the sidebar and `website/src/lib/apps.ts`.
  `website/src/content/docs/guides/example-apps/index.mdx:1`

### 2. a browser test drives chat.js and poll.js in Chromium against the served apps

**Does:** proves the browser half of both apps in a real browser, under the apps' own CSP.

- [ ] Chat: two browser contexts join the lobby; a line sent in one appears once in the other; a
  line with `<b>` shows as text; a line sent too fast shows the `problem` notice.
  `tools/nv/test/realtime-browser.test.ts:1`
- [ ] Chat: the server is stopped and started again; the page reconnects by itself, and the history
  shows each line once (the `id` check in `apps/chat/public/chat.js:1`).
- [ ] Poll: two pages of one poll; a vote in one moves the bar in the other; a second vote from the
  same browser moves its vote and the total stays the same. `apps/poll/public/poll.js:1`
- [ ] Neither page logs a console error or a CSP violation. `tools/nv/test/chromium.ts:1`
- [ ] Add the suite to CI's `browser` job and the paths to `browser` in
  `tools/nv/cmd/ci-changes.ts:181`, and run it once by name with `NV_BROWSER=1`.

### 3. a guide page says when to use a WebSocket and when an event stream, and links both apps

**Does:** gives the reader the choice before the code.

- [ ] `website/src/content/docs/guides/how-to-use/live-updates.mdx:1`: when the server only sends
  (event stream) and when both sides send (WebSocket); the shared model (the upgrade starts an
  isolate, `args:`, topics, a slow subscriber is closed, one process); the `Origin` check and
  `trusted_proxies` behind a proxy; links to both apps and to `Core\Socket::upgrade`,
  `Core\Sse::upgrade` and `Core\Topic::publish`. Prose per AGENTS.md § *Text an end user reads*.
- [ ] Add it to the `how-to-use` sidebar group in `website/astro.config.ts:1` and its index.
- [ ] Link it from the two app pages' `## Read more`.

### 4. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written by the interactive session of 2026-11-05. `apps/chat`, `apps/poll`, their pages and
`website/src/lib/apps.ts` are on disk and may still be uncommitted; `git status -- apps website`
says which. Two writers were making both apps production-ready (decisions 3, 4 and 6) when this goal
was written. The three Novis fixes in *Why* are committed apart from the apps.

### Next

**Stage 1**, over `apps/chat/`, `apps/poll/` and the two pages; no gate.

- [ ] Reconcile `deploy/`. `apps/chat/deploy/Caddyfile:1`
- [ ] `bun nv site --check apps --nvs target/release/nvs.exe`. `tools/nv/cmd/site.ts:1913`
- [ ] The prose and website checks. `website/src/content/docs/guides/example-apps/chat.mdx:1`
- [ ] The real test output on each page. `website/src/content/docs/guides/example-apps/poll.mdx:66`
- [ ] Commit. `website/src/content/docs/guides/example-apps/index.mdx:1`

### Backlog

- A screenshot of each app on its page, if `bun nv site --shoot` grows an app shot that is not the
  debug viewer (`tools/nv/cmd/site.ts:200`).
