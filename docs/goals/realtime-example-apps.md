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
    title: "a request's wall_time no longer ends a WebSocket or an event stream it upgraded"
    status: open
    context: [docs/rules/concurrency/a-connection-is-a-root-isolate.md, docs/rules/http-server/connection-bounds-are-finite.md, website/src/content/docs/guides/production/security-checklist.mdx]
  - id: 5
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
   inline script or style is added to either app. One line of configuration covers every response,
   error pages and static files too, where a `Response::setHeader` covers only the responses the
   program writes. The test reads the setting with `Core\Config::get`, because `Test::request`
   returns only the headers the program sets (`apps/chat/src/AppTest.nvs:87`). Poll sets its policy
   with `setHeader` in `apps/poll/public/index.nvs` and moves to this.
4. **Identity:** chat keeps the visitor's name in `Core\Session`, read by the upgrade request, and
   passes only the name and the room to the connection through `args:`. Poll gives each browser a
   long-lived voter id, and a second vote from the same browser moves that vote. A login is the
   reader's to add, and each page says so.
5. **Topic names are built from checked values only** (a room from the app's own list, a poll id from
   a `uint` route parameter), and the published value carries no secret.
6. **`deploy/` is one shape in both apps:** `deploy/Caddyfile`, `deploy/nginx.conf` and
   `deploy/production.toml`, which holds only what production changes and is passed after the base:
   `nvs serve public/index.nvs --config nvs.toml --config deploy/production.toml`. This is poll's
   shape. Chat's `production.toml` is two `[[include]]` lines over `deploy/server.toml`, because an
   included file overrides the file that includes it (`nvs config dump --origin` shows it). That is
   three files to read where two flags do, so chat's `server.toml` becomes its `production.toml`, and
   its page, README and `nvs service install` line follow. Each page's `## Deploy it` walks through
   exactly these files.
7. **Topics stay in one process.** Neither app adds a broker. Each page says that several machines
   need a message queue in application code (`rule:core-classes/topic`).
8. **Tests:** unit tests use the in-memory `scratch` database; a live test (`#[Test(server: true)]`)
   uses the app's own database and deletes the rows it wrote.
9. **The browser test** (stage 2) is a new `tools/nv/test/realtime-browser.test.ts` on the shared
   driver `tools/nv/test/chromium.ts`, run like `registry-browser.test.ts`: a copy of each app under
   `.agent-tmp/`, served by `nvs serve` on a free port, `NVS_BIN` honoured, skipped without Chromium
   unless `NV_BROWSER` is set. It is added to CI's `browser` job and to the `browser` list in
   `tools/nv/cmd/ci-changes.ts`, with `apps/chat/` and `apps/poll/`.
10. **Lanes:** stage 1 first. Stages 2, 3 and 4 touch disjoint files and may run as side goals once
    stage 1 is committed.

## Stages

### 1. both apps land with one production setup, and the website's app check is green over them

**Does:** reconciles the two `deploy/` folders, runs the website's checks over both apps, and commits
the apps with their pages.

- [ ] Make `apps/chat/deploy/` and `apps/poll/deploy/` the same shape (decision 6), and each page's
  `## Deploy it` match its files. `apps/chat/deploy/production.toml:1`, `apps/poll/deploy/production.toml:1`
- [ ] Move poll's policy to `[http.headers]`, and its test to `Core\Config::get` (decision 3).
  `apps/poll/public/index.nvs:1`
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

### 4. a request's wall_time no longer ends a WebSocket or an event stream it upgraded

**Does:** settles which bound ends a long-lived connection, so the security checklist's advice and the
two apps work together.

Found on 2026-11-05: with `[limits] wall_time = "2s"`, `curl -N /polls/1/events` closes after 2.1
seconds, and every `EventSource` reconnects over and over. The security checklist tells operators to
set `wall_time`, and `limits:` on an upgrade only narrows, so an app cannot exempt its streams. A
connection already has its own finite bounds, `[server.connection] idle_timeout` and `max_lifetime`
(`rule:http-server/connection-bounds-are-finite`), and is a root isolate with its own budget
(`rule:concurrency/a-connection-is-a-root-isolate`). The session decides by the priority ordering and
records the decision in those rules. The safe default: `wall_time` bounds the request that upgrades,
`max_lifetime` bounds the connection, and `memory` and CPU time apply to both.

- [ ] Reproduce it with a Rust test over a served event stream and a WebSocket under a short
  `wall_time`. `crates/nvs-server/src/serve.rs:2539`
- [ ] Fix it, or record why it stays, in both rule fragments, and run `bun nv rules --render`.
  `docs/rules/concurrency/a-connection-is-a-root-isolate.md:1`
- [ ] Make the checklist say what bounds a connection.
  `website/src/content/docs/guides/production/security-checklist.mdx:25`
- [ ] Remove the note in each app's `deploy/production.toml` that explains why it sets no `wall_time`,
  once the fix makes the note untrue. `apps/poll/deploy/production.toml:1`

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Both apps and their pages are committed with production setups: a session (chat), one vote per
browser (poll), a strict CSP, and `deploy/` with Caddy, nginx and production settings. Each passes
`nvs test .` with 22 tests, real WebSocket and event-stream tests among them, and `bun nv site
--check apps` passes for both. Its only red is Paperwork's three PDF copies, which a binary built
after `d6910ac5a6` writes differently; that is not this goal's. The two apps still differ in where
the CSP is set and in the shape of `deploy/` (decisions 3 and 6).

### Next

**Stage 1**, over `apps/chat/`, `apps/poll/` and the two pages; no gate.

- [ ] Chat's `deploy/` takes decision 6's shape. `apps/chat/deploy/production.toml:1`
- [ ] Poll's policy takes decision 3's place. `apps/poll/public/index.nvs:1`
- [ ] `bun nv site --check apps` and the prose and website checks. `tools/nv/cmd/site.ts:1913`
- [ ] The real test output on each page. `website/src/content/docs/guides/example-apps/chat.mdx:1`
- [ ] Commit. `website/src/content/docs/guides/example-apps/poll.mdx:1`

### Backlog

- `[http.headers] content_security_policy` that already names `frame-ancestors 'none'` is sent with
  Novis's own `frame-ancestors 'none'` appended a second time; harmless, and both apps leave it out
  of their policy for now (`crates/nvs-server/src/secure.rs:1`).
- Whether `connect-src 'self'` allows `wss:` to the same host in every supported browser; CSP Level 3
  says it does, and stage 2's Chromium test shows it for Chromium (`apps/chat/nvs.toml:14`).

- A screenshot of each app on its page, if `bun nv site --shoot` grows an app shot that is not the
  debug viewer (`tools/nv/cmd/site.ts:200`).
