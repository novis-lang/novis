---
looper: 1
title: "the debug viewer wears the website's design, answers per-route and per-statement questions, and opens in the editor in one click"
summary: "Reached when the viewer has the website's palette and both themes in the system's own fonts on one set of shared components, a frame with a rail, a command palette and a live state, a request list with filters, summary tiles, a time split per row and a repeated-query badge, a request page with tabs for its timeline of all eight event kinds, its request, its response, its queries, dumps, logs and coverage, a Routes and a Queries view, a visible state for lost records, nvs debug link prints the link, the page is served under a token path that only the editor may frame, the VS Code extension opens the viewer in a panel or the browser from one button, a file:line in the panel opens in the editor, lenses show what a route, a dump and a throw site recorded, and the rules, the feature proofs, the screenshots and the website say so."
context: [viewer/design/reference.html, docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md, viewer/src/app.tsx, viewer/src/api.ts, crates/nvs-cli/src/debug/listen.rs]
stages:
  - id: 1
    title: the rules say how the viewer looks, what its new views answer, and how the editor opens it
    status: done
    context: [docs/agent/doc-style.md, docs/rules/ide/the-extension-builds-no-ui-the-editor-already-has.md, docs/rules/ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site.md, docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md, website/src/styles/custom.css]
  - id: 2
    title: the viewer has the website's palette in the system's fonts, a frame with a rail and a command palette, and one set of shared components
    status: done
    context: [viewer/src/styles/tokens.css, viewer/src/keys.ts, viewer/src/theme.ts, viewer/build.ts, tools/nv/cmd/viewer.ts, website/src/styles/custom.css]
  - id: 3
    title: every list view is the shared list, and every view wears the new design
    status: done
    context: [viewer/src/views/requests.tsx, viewer/src/views/stream.tsx, viewer/src/views/exceptions.tsx, viewer/src/views/search.tsx, viewer/src/views/compare.tsx]
  - id: 4
    title: the index holds each request's time by kind and its repeated statements, and the API filters and summarises
    status: done
    context: [crates/nvs-cli/src/debug/index.rs, crates/nvs-cli/src/debug/api.rs, crates/nvs-cli/src/debug/ingest.rs, crates/nvs-cli/tests/debug_api.rs, crates/nvs-lsp/src/recorded.rs]
  - id: 5
    title: the request list has filters, summary tiles, a time split per row and a peek pane
    status: done
    context: [viewer/src/views/requests.tsx, viewer/src/router.ts, viewer/test/requests.browser.test.ts]
  - id: 6
    title: the request page has a header, a time split and tabs for its timeline, request, response, queries, dumps, logs and coverage
    status: done
    context: [viewer/src/views/request.tsx, viewer/src/views/timeline.tsx, viewer/src/views/value-tree.tsx, viewer/src/views/compare.tsx, viewer/test/request.browser.test.ts]
  - id: 7
    title: the Routes view, the Queries view, the lost-records state and the settings page
    status: open
    context: [crates/nvs-cli/src/debug/api.rs, viewer/src/views/saved.tsx, viewer/src/views/editor.tsx, viewer/src/views/stream.tsx]
  - id: 8
    title: nvs debug link prints the link, and the page is served under a token path that only the editor may frame
    status: open
    context: [crates/nvs-cli/src/debug.rs, crates/nvs-cli/src/debug/secure.rs, crates/nvs-cli/tests/debug_viewer.rs, viewer/src/router.ts, viewer/src/editor-link.ts, docs/agent/coding.md]
  - id: 9
    title: the extension opens the viewer in a panel or the browser from one button, and a site in the panel opens in the editor
    status: open
    context: [editors/vscode/package.json, editors/vscode/src/extension.ts, editors/vscode/src/profile.ts, editors/vscode/src/viewer.ts, editors/vscode/DEVELOPING.md, crates/nvs-lsp/src/server.rs, docs/rules/ide/contributions-are-frozen-and-only-ever-added.md, docs/rules/ide/the-request-set-is-closed.md]
  - id: 10
    title: a lens shows what a route, a dump and a throw site recorded, and opens it in the viewer
    status: open
    context: [crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/recorded.rs, crates/nvs-lsp/src/lens_rows.rs]
  - id: 11
    title: the feature proofs, the screenshots, the help and the website show the new viewer
    status: open
    context: [docs/agent/prose.md, website/src/content/docs/tools/debug-viewer/index.mdx, website/src/content/docs/tools/editor/index.mdx, website/src/data/changes.json, tools/nv/cmd/site.ts]
  - id: 12
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: viewer-built
    stage: 2
    name: "the viewer's types hold, its committed bundle is what a build gives, and its unit tests pass"
    run: [bun, nv, viewer, --verify]
    timeout: 900
  - id: api
    stage: 4
    name: "the request list filters by method, status, route, duration and exception, and the summary, the time by kind and the repeat count are what the fixture's records give"
    run: [cargo, test, --test, debug_api]
    timeout: 1800
  - id: embed
    stage: 8
    name: "a token path serves the page and the API with no cookie and may be framed by the editor alone, a wrong token is 401, and every other path keeps frame-ancestors 'none'"
    run: [cargo, test, --test, debug_viewer]
    timeout: 1800
  - id: extension
    stage: 9
    name: "the extension's headless tests pass with the new commands, the panel's page and its message checks"
    run: [npm, --prefix, editors/vscode, run, "test:headless"]
    timeout: 1800
  - id: shots
    stage: 11
    name: "every screenshot on the website is what the viewer shows now"
    run: [bun, nv, site, --check, shots]
    timeout: 1800
  - id: proofs
    stage: 11
    name: "every feature the goal's work reaches has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, debug-viewer-flagship]
    timeout: 600
  - id: close
    stage: 12
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, debug-viewer-flagship]
    timeout: 900
---

# the debug viewer wears the website's design, answers per-route and per-statement questions, and opens in the editor in one click

## Why

The viewer is the window a developer watches a Novis program through, and
`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact` holds it to the standard of a
tool kept open all day. Its function meets that standard. Its look does not: GitHub's
blue on white, one weight and one size for nearly all text, underlined text links for navigation, a `<select>` for the theme, and one table per
view (`website/snippets/tools/debug-viewer/requests.png`). It shares nothing with the website's
palette, so the product's two faces do not look like one product.

The structure under the look has the same gaps:

- One stylesheet of 1156 lines (`viewer/src/style.css:1`), and no shared component. The virtual list
  and its `j`/`k` keys are written four times (`viewer/src/views/requests.tsx:230`,
  `stream.tsx:246`, `exceptions.tsx:110`, `timeline.tsx:149`).
- The request list takes no filter: `GET /api/requests` reads only a page position
  (`crates/nvs-cli/src/debug/api.rs:319`).
- A request's recorded headers and bodies are in the index (`crates/nvs-cli/src/debug/index.rs:101`)
  and show only in the compare view. The request page never renders them.
- The runtime records `cache`, `session` and `file` events (`crates/nvs-runtime/src/spool.rs:573`),
  and the viewer's types and timeline labels know five kinds (`viewer/src/api.ts:84`,
  `viewer/src/views/timeline.tsx:46`).
- No view answers "which route is slow" or "which statement runs most", though the index has the
  columns and `events_site` (`index.rs:151`).
- Lost records (`dropped`, `unparseable`, `quarantine`) show only under `/stream`
  (`viewer/src/views/stream.tsx:508`).
- `t` is bound twice: the theme (`viewer/src/app.tsx:64`) and *Make a test*
  (`viewer/src/views/request.tsx:96`).

The editor's half is thin too. The extension has no command for the viewer. The query lens opens
`http://127.0.0.1:8137/requests/<trace>` with no token (`crates/nvs-lsp/src/recorded.rs:325`), so its
click answers `401` in a browser that has no cookie yet. No command prints the link without starting
the service (`crates/nvs-cli/src/debug.rs:118`).

## Standing decisions

**The user's calls.** The viewer becomes a flagship feature with a well-designed UI and features. Its
design is the website's: colours and theming. The extension gets a button that opens the viewer with
one click, inside VS Code or in a real browser. The goal also looks for what the editor, the UI and
Novis can do better here. The viewer embeds no font and draws with the system's own, chosen to look
good. Every other decision below is the session's that wrote this goal.

**`viewer/design/reference.html` is the drawing every stage builds toward.** It shows the request
list, one request and the editor at the size and density meant. It is not markup to copy: it uses
inline styles, which the viewer's `Content-Security-Policy` refuses. A value that changes at run
time, such as a bar's left edge and width, is set through the CSSOM (`el.style.setProperty`), which
the policy allows and Solid's `style` object compiles to. Stage 11 deletes the file. The memory
figures, the handler link and the response size in the drawing arrive with the goal
`deeper-debug-records`. Each view leaves their place and draws one only when its record has the
field.

**The palette is the website's block, copied once.** `viewer/src/styles/tokens.css` starts with the seven
colours of `website/src/styles/custom.css:66` (`--nv-crimson`, `--nv-orange`, `--nv-gold`,
`--nv-blue`, `--nv-ink`, `--nv-fog`, `--nv-ground`) and derives every other colour with `color-mix()`
as the website does, in both themes, with the Display-P3 copy. The viewer names roles (`--surface`,
`--line`, `--text`, `--muted`, `--link`, `--wash`, `--ok`, `--warn`, `--bad`), never Starlight's
`--sl-*` names. A colour has one meaning, as on the website: green is ok, blue is interaction and
info, orange is warn, red is an error, gold is a highlight such as a repeated query. The event kinds
take the website's diagram tones: `query` blue, `http` violet, `cache` and `session` green, `file`
gold, `gc` orange, `spawn` crimson, and time in the program's own code is the faint gray. The
signature gradient paints the header's hairline and the logo tile, and never text in a list. Crimson
fills only the one primary action of a page, *Replay*. Each text colour keeps WCAG AA on every
surface it is drawn on, which `the-viewer-shows-the-stream…` already demands. `bun nv viewer --check`
gains a test that the seven palette values in the two files are equal, so a re-tint of the website
reaches the viewer.

**The viewer draws with the system's fonts and ships none.** This is the user's call. No font file
is embedded, served or fetched, the stylesheet has no `@font-face`, and the service gains no font
route. `tokens.css` declares two stacks once, and no other rule names a face:

- `--sans: 'Segoe UI Variable Text', system-ui, -apple-system, 'Segoe UI', Roboto, 'Noto Sans',
  sans-serif` for all text. The first name gives Windows 11 its variable face where it is installed.
- `--mono: ui-monospace, 'Cascadia Mono', 'SF Mono', Menlo, Consolas, 'DejaVu Sans Mono',
  'Liberation Mono', monospace` for code, paths, statements and ids.

What makes a system face look made for the page is set in `base.css`: a scale of five sizes (11, 12,
13, 15 and 20 px), text at weight 400 and 600 with 700 for the brand alone, code at 400 and 700,
`font-variant-numeric: tabular-nums` on every figure, `font-variant-ligatures: none` on code so
`!=` stays two characters, a letter spacing of `-0.01em` from 15 px up, and
`font-optical-sizing: auto`. No weight between these is used, because Consolas and Menlo have none.
Buys: nothing added to the binary, no licence notice, no build step for fonts, and text drawn on the
first frame. Costs: the viewer looks a little different on each system, its letters are not the
website's, and on Linux it is as good as the fonts installed there. The palette, the gradient and
the components carry the likeness to the website. Declined: a font file in the binary, a font from
a network, and a face the stack names that a system has to install.

**The frame.** A left rail of icon and label (Requests, Routes, Queries, Errors, Logs, Stream, and
Settings at its foot), a top bar with the logo, the search box, the live state and the theme button,
and the view. The icons are inline SVG paths in one module, drawn on a 16 pixel grid with one stroke
width, and no icon package is added. The rail shows a count on Errors (groups seen since the tab
opened). The live state is one pill: *Live* with the replay target's port when a server started the
service, *Recording stopped* when the service runs alone, and it turns to the warn colour with a
count when records were lost. Below 900 pixels the rail shows icons alone.

Stage 2 built it this way, and later stages keep it. The rail is 11rem wide, an icon beside its
label, with the saved filters under the views, because a name does not fit under an icon. Below 900
pixels the labels and the saved filters are hidden from sight, and the palette lists the filters.
Routes and Queries join the rail in stage 7, when their views exist, and the Errors count and the
pill's states come with stage 7's health route. Search is not a rail item: the top bar's box and
the palette open it. Settings at the rail's foot opens the editor settings dialog until stage 7's
settings page. The theme button walks System, Dark and Light, and its `aria-label` names the
current one.

**One key map.** `viewer/src/keys.ts` becomes the map every view registers with, and the `?` dialog
prints it, so a key cannot be bound twice. `Ctrl+K` (`Cmd+K` on macOS) opens the command palette:
views, saved filters, the actions of the page, and the search. `/` still focuses the search box. `t`
is *Make a test* alone. The theme has no single key: it is the top bar's button, a palette action
and a setting.

**Shared components, in `viewer/src/ui/`.** `List` (the one virtual list, with selection, the row
keys, follow and pause), `Chip`, `Method`, `Status`, `Tabs`, `Dialog`, `Toast`, `Empty`, `Tile`,
`Spark`, `Meter`, `Icon`, `Site` (moved from `viewer/src/site.tsx`). Each view's file holds its
columns and its data, never list mechanics. The stylesheet is split by layer into
`viewer/src/styles/` (`tokens.css`, `base.css`, `ui.css`, one file per view) and imported from
`main.tsx`, so the bundle stays one CSS file.

**The names stage 2 gave, which later stages use.**

- **Styles** are `viewer/src/styles/`: `tokens.css`, `base.css`, `ui.css` (frame and components),
  and one file per view, imported in order by `viewer/src/main.tsx`. Each role is a `light-dark()`
  pair, which Bun's CSS build lowers to `--buncss-light` and `--buncss-dark` toggles keyed on
  `prefers-color-scheme` and `data-theme`. The roles are the rule's, plus `--accent` and
  `--highlight` (blue and gold as drawn, never text), `--primary` (the crimson fill), `--k-<kind>`
  and the `.k-<kind>` fill classes.
- **Components** are `viewer/src/ui/`: `icon.tsx` (`Icon`), `badges.tsx` (`Chip`, `Method`,
  `Status`, `Repeat`), `tabs.tsx`, `dialog.tsx`, `toast.tsx` (`toast()`, and `Toasts` rendered once
  by the frame), `empty.tsx`, `tile.tsx` (`Tile`, `Spark`, `Meter`), `site.tsx` and `palette.tsx`.
- **Keys**: `keys.ts` has `bind()`, which throws on a key bound twice, `dispatch()`, `label()`,
  `VIEW_KEYS` and `ignored()`. The frame binds `Mod+k`, `/` and `?`. A view's own keys are listed in
  `VIEW_KEYS` until its list binds them.
- **Palette**: a page adds actions with `onCleanup(addCommands([...]))`. Its list is in the DOM only
  while it is open, because the browser tests count every `[role=option]` and focus the first
  `[role=listbox]` on the page.
- **Test hooks**: the theme is `button.theme` with `aria-label` `Theme: <name>`, `.editor-open` is
  the rail's Settings item, and the hostile test allows `svg.icon[aria-hidden]`.

**The names stages 3 and 4 gave, which later stages use.** Stage 3:

- `viewer/src/ui/list.tsx`: `List` (virtual rows, selection by key, the shared keys, a view's own
  keys through `keys`, the follow bar, `actions`, `empty`, `cols` and `grid`) and `createFeed`
  (newest page, follow, pause, arrived count, older pages). A list that reloads its first page on
  each watermark, as both exception lists do, passes `rows`, `more`, `onEnd`, `loaded` and `error`
  to `List` and keeps its own loader.
- `views/requests.tsx` exports `RequestCells`, `RequestColumns`, `REQUEST_ROW` (38), `age()` and
  `createNow()`. The request grid is `.request-grid`. `views/feed.tsx` holds `Feed`, `KindDot`,
  `LevelChip`, `RecordDetail` and the stream helpers, and `views/filter-form.tsx` holds `FilterForm`.
- Only four event kinds have a `--k-*` tone in `tokens.css`. `stream.css` maps the others onto
  roles with `.rk-<kind>`.

Stage 4:

- The index is schema 10. `requests` has `db_us`, `http_us`, `cache_us`, `session_us`, `file_us`,
  `gc_us`, `spawn_us` and `repeats`, kept by `requests_search_in` and `events_count`, and the
  index `requests_route`. The editor's lens reads no column that changed and checks no schema.
- A request row's JSON has `time_us` (`{query, http, cache, session, file, gc, spawn}`) and
  `repeats`. `api::ROW_LEN` is the column count of `api::ROW`, which every query that selects
  more after it uses.
- `GET /api/requests` takes `method`, `status` (`404` or `5xx`), `route`, `min_us` and
  `threw=true|false`. A filtered page can be short with `older` set (*The API's new answers*).
- `crates/nvs-cli/src/debug/summary.rs` serves `/api/summary?until=`, `/api/routes`,
  `/api/statements` and `/api/health`. `viewer/src/api.ts` has `summary()`, `routes()`,
  `statements()`, `health()`, their types, `TimeByKind` and `EventKind` (all eight).
- `timeline.tsx`'s `KIND_NAMES` names `cache`, `session` and `file`, but they have no tone yet.
  `RECORD_KINDS`, the stream's filter list, still lacks them; the service takes all eight.

**The names stage 5 gave, which later stages use.**

- `viewer/src/ui/menu.tsx`: `ChipMenu`, a chip whose menu is a native `popover="auto"` of
  `menuitemradio` buttons, "Any" first. Its place is set through the CSSOM when it opens.
- `views/requests.tsx` exports `ListFilter`, `listFilter()`, `listQuery()` (keys in one order:
  `method`, `status`, `route`, `min_us`, `threw`), `listLink()` (the list's last link, which the
  request page's `Escape` opens), `split()` (a row's time as `Part`s, `own` last, scaled down
  when the kinds overlap past the duration) and `RequestCells`' `scale`. Stage 7's Routes view
  opens the filtered list with `viewLink("requests", listQuery({ method, route }))`.
- `RequestList` takes the query, and `app.tsx` keys it on `query()`, so a new filter is a new feed.
- `Feed` has `busy` and `batch`, and `List` has `onShown`. A list whose page is short and has
  `older` loads the next range at once, until the screen is full or none is left.
- The tiles are three: requests, duration and errors. The memory tile waits for
  `deeper-debug-records`. They do not read again while the list is paused.
- The peek pane fetches `/api/request` 120 ms after the selection settles and draws at most 24
  events, function calls left out.
- A filtered `since` page is cut only when what arrived after `since` passes the scan's floor
  (`crates/nvs-cli/src/debug/api.rs` `page`).

**The names stage 6 gave, which later stages use.**

- `views/request.tsx` exports `TABS`, `TabId` and `tabLink(row, tab, hash)`. The Timeline is the
  page's own link, and every other tab is `?tab=<id>`. A link that ends in `#exception` opens the
  Exception tab. Stage 10's dump lens opens `tabLink(row, "dumps", "#dump-<id>")`.
- The header is `.req-head`: the title `h2.title` is the method and the path, and the line under
  it, `.req-crumb`, has the route, the age, the exception and the ids. Four tiles: duration,
  queries, dumps and log lines. The memory and response tiles wait for `deeper-debug-records`.
- The Coverage tab is always in the row, with an `Empty` when nothing was recorded, so a tab's
  place never moves. The Exception tab is there only when the request threw. The Timeline tab
  also lists the profiles and the copy sites, which have no tab of their own.
- `Timeline` takes `reveal(kind, id)`, which opens the dump's or the log line's tab at it. It is a
  waterfall: one row per event, or one `group-<first id>` row for a statement that ran twice or
  more at one site. `lanes()` is gone.
- `views/request/`: `input.tsx`, `output.tsx`, `queries.tsx` (`queryGroups()`), `body.tsx`
  (`HeaderTable`, `BodyView`) and `curl.ts` (`curl()`, `quote()`). A JSON body is drawn with the
  value tree unless a key starts with `$`, which the tree reads as a dumped value's own key; then
  it is text. `compare.tsx` exports `bodyNote()`, the one home of the sentences for a body that
  was not read, was cut or was withheld.
- *Copy as curl* takes the address from the recorded `Host` header, or `localhost`, and leaves out
  a body that is binary, cut or withheld, and its toast says so.
- `RECORD_KINDS` has all eight event kinds, and `stream.css` has `.rk-cache`, `.rk-session` and
  `.rk-file`.

**Motion is transform, opacity and colour only,** 140 ms, and none under
`prefers-reduced-motion`. A new row fades in, and nothing on screen moves when a record arrives
while paused, as the rule already says.

**The browser tests keep their hooks.** A class a test selects (`.replay-action`, `.follow-state`,
`.arrived`, `.cmp-count`, `.source-line`, `.mark-state`, `.log-message`, `dialog.keys`,
`dialog.ask`) stays on the element that carries its meaning. A test whose element is gone is
rewritten in the slice that removes it, selecting by role and accessible name. Every stage that
changes a view runs that view's browser test by name:
`NV_BROWSER=1 bun test viewer/test/<view>.browser.test.ts`.

**The index holds what the list draws, computed once at ingest.** `requests` gains `db_us`,
`http_us`, `cache_us`, `session_us`, `file_us`, `gc_us` and `spawn_us`, the summed durations of the
request's events by kind, and `repeats`, the largest number of times one statement at one call site
ran in the request. The trigger that keeps `queries` (`crates/nvs-cli/src/debug/index.rs:318`) keeps
them. The schema version rises, and the index is rebuilt from the spool, which is what a cache is
for. A repeated statement is defined once, by
`rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`, so the editor's lens and the
viewer's badge agree. "Your code" is the duration less the summed kinds, never below zero. Spends:
eight integers per request row.

**The API's new answers are bounded reads.**

| Route | Answers | Bound |
|---|---|---|
| `GET /api/requests` | the page, filtered by `method`, `status` (a code or a class such as `5xx`), `route`, `min_us`, `threw`, each a bound parameter | the page size, and a filtered page reads at most 20,000 ids below its cursor |
| `GET /api/summary` | twelve buckets over the last five minutes of count and errors, and p50, p95 and max duration | the newest 10,000 rows of the window, and the answer says when it was cut |
| `GET /api/routes` | per method and route: count, error count, mean queries, max duration, last seen, p50 and p95 | the newest 10,000 requests and 200 routes by count; the percentiles read each route's newest 500 durations through a new index on `(method, route, start_us)` |
| `GET /api/statements` | per call site: the newest statement text, count, total and max time, requests reached, requests where it repeated | the newest 100,000 event ids and 200 sites by total time |
| `GET /api/health` | the counts of `dropped`, `unparseable` and `quarantine` rows, and whether a replay target is known | three counts |

A percentile is computed in Rust from the sorted durations, because SQLite has none. No route
changes a record.

Stage 4 bounded each read by rows, never by the index's size, because the rule declines an
aggregate with no bound. A filtered request page reads at most 20,000 ids below its cursor, and
when it stops short of its limit, `older` names the floor it stopped at, so the viewer loads the
next range on scroll as it does a full page. Declined: an index per filter column, because
`ORDER BY id` over a status range or a duration bound then sorts every match. The routes count
the newest 10,000 requests, and the statements the newest 100,000 event ids. Each answer has
`partial` when older rows were not read, and a route has `durations`, the count its percentiles
stand on. The fixtures under `viewer/test/fixtures/` gain one file per new route, written by
`crates/nvs-cli/tests/debug_api.rs` and checked by `viewer/test/api.test.ts`, as the rule demands.

**The request page is tabs, and a body is never run.** Timeline, Request, Response, Queries, Dumps,
Logs, Coverage, and Exception when it threw. The tab is part of the link
(`/requests/<trace>/<span>?tab=queries`). Request and Response show headers as a table and the body
as text. A JSON body is parsed in the page and drawn with the value tree. An HTML body is shown as
text. Declined: drawing an HTML response in a frame, which runs it, and drawing an image body. The
Queries tab groups statements by call site, marks a repeated one in gold with its count, and names
the site as a link. *Copy as curl* builds the command from the recorded input, with `[redacted]`
where the record has it, as a test made from the request does.

**The page is embedded under a token path, and only the editor may frame it.** A framed page is
cross-site to its frame, so the browser neither stores nor sends the `SameSite=Strict` cookie there.
The service therefore also serves the page, the bundle and the API under
`/embed/<token>/`, compared in constant time, with no cookie set or read. A response under that
prefix carries `frame-ancestors vscode-webview:`; every other response keeps `frame-ancestors
'none'`. The page reads its base path from `location.pathname`, and the router, the fetches and the
event stream put it in front of every path. `Referrer-Policy: no-referrer` already keeps the path
out of any request, and the listener writes no path to a log. Spends: the token is in the frame's
URL, readable by the page's own script, which can already read everything the token opens. An
`Origin` under the prefix must be the listener's own or absent, as today. Declined: a `SameSite=None`
cookie (it gives every site on the machine the cookie), VS Code's Simple Browser (it cannot pass a
click to the editor, and it is cross-site too), and passing the API through the extension host (a
second transport to keep correct).

**Embedded, the page and the panel exchange three messages and nothing else.** The page sends
`{type: "open", file, line, column}` to `window.parent` in place of following a `vscode://` link,
and only when its base path is the embed prefix. The panel sends `{type: "theme", value}` and
`{type: "go", path}`. The page takes a message only from `window.parent` with a `vscode-webview:`
origin, and `go` only for a path `route()` reads (`viewer/src/router.ts:77`). The panel takes a
message only from the frame's origin. The extension host opens a file only when it exists under a
workspace folder, after the viewer's own path mappings, and runs nothing.

**`nvs debug link` prints the link and starts nothing.** It reads `[debug] listen` and the token
file as the service does, prints the link with `?token=`, and says on standard error when no
service holds `debug/service.lock` or when `[debug] store` is off. `--json` prints `url`, `embed`,
`running` and `recording`. A missing token file is created exactly as the service creates it
(`crates/nvs-cli/src/debug/secure.rs:62`), so the link is the one the next service honours.

**The extension draws a frame and nothing in it.** Two commands, `nvs.openDebugViewer` (*Open Debug
Viewer*) and `nvs.openDebugViewerInBrowser`, and one setting, `nvs.debugViewer.openIn` (`editor` by
default, or `browser`), which decides what the button does. The button is the first command in
`editor/title` for an `nvs` or `nvst` document and for `nvs.toml`, with the pulse icon. The panel is
one webview, `nvs.debugViewer`, whose page is one `<iframe>` of the embed link and a script that
passes the three messages. Its policy admits that one frame source and one nonce script. It follows
`rule:ide/the-extension-builds-no-ui-the-editor-already-has`: the editor has no surface that shows a
local page and passes a click back, and the panel renders nothing of its own. The extension learns
the link from the language server, which already knows the data folder and `[debug] listen`
(`crates/nvs-lsp/src/server.rs:2432`): a new request `nvs/debugViewer` answers `url`, `embed`,
`running` and `recording`. When no service runs, the command offers *Start nvs debug*, which runs
`nvs debug` as a task of the `nvs` task type. When `store` is off, it says which line of `nvs.toml`
turns recording on and opens the file. The browser command goes through
`vscode.env.asExternalUri`, so a remote window opens the forwarded address. A second click on the
button reveals the open panel. A lens that opens a request calls `nvs.openDebugViewer` with the
path, so the open panel moves to it with `go` and no second tab opens. Stop: if the host test shows
that VS Code does not draw an `http://127.0.0.1` frame in a webview, the panel is dropped after one
attempt, both commands open the browser, and the rule says so.

**Three new lenses read the index, as the query lens does.** All are under `nvs.codeLens.enable`,
read `debug/index.sqlite` read-only without waiting (`crates/nvs-lsp/src/recorded.rs:7`), and show
nothing when there is no index.

- Above a `#[Core\Route]` handler: its recorded requests, the p50 duration and the newest failing
  status with its age. It keys on the method and the route pattern, so an edit to the file cannot
  move it. A click opens the request list filtered to the route.
- Above a `Core\Debug::dump` call: the newest value dumped at that file and line, as one line of at
  most 80 characters from the already redacted tree. A click opens that request's Dumps tab.
- Above a `throw` whose site an exception group names: how often it was thrown and when last. A
  click opens the group.

A dump or a throw lens shows only while the file is not newer than the record, because a line
number means nothing after an edit above it. Declined: remapping a recorded line through the file's
edits.

**Lanes.** Stages 2, 3, 5, 6 and 7 share `viewer/` and run in order. Stage 4 touches only
`crates/nvs-cli/src/debug/` and its tests, and can run beside stage 2 or 3. Stage 8 needs stage 2's
router. Stage 9 needs stage 8's link, and stage 10 needs stage 9's command. The goal is on
the chain by the user's call, in front of `deeper-debug-records`, whose viewer items build on
`viewer/src/ui/`.

**What stops the goal.** One attempt per item and two sessions per stage. A stage that cannot end in
two sessions records what is left in the handoff and the next stage starts. The count that decides
is the bundle. If `viewer/dist/viewer.js` passes twice its size at the goal's first commit, the
stage that crossed it says in the rule what the bytes buy.

## Stages

### 1. the rules say how the viewer looks, what its new views answer, and how the editor opens it

- [x] `docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md:28`: *The
  baseline every view has* gains the frame, the one key map and the command palette. The paragraph
  on the front end gains `viewer/src/ui/` and the split stylesheet, and says that the viewer ships
  no font.
- [x] `docs/rules/testing/the-viewer-wears-the-websites-palette.md:1`: the palette block copied
  once and checked equal, the role names, one meaning per colour, the tone of each event kind, where
  the gradient and crimson may appear, the two system font stacks and the type scale, motion.
  Declined: Starlight's variable names, an icon package, an embedded or fetched font, a colour
  picked per view.
- [x] `docs/rules/testing/a-request-page-shows-what-was-recorded-and-runs-none-of-it.md:1`: the tabs,
  the tab in the link, bodies as text, JSON as a tree, the repeated-statement mark, *Copy as curl*.
  Declined: an HTML body in a frame, an image body drawn.
- [x] `docs/rules/testing/the-viewer-answers-per-route-and-per-statement.md:1`: the five API answers
  and each bound from the table above, the eight `requests` columns and their trigger, the percentile
  in Rust. Declined: an aggregate with no bound, a second store for aggregates.
- [x] `docs/rules/testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time.md:7`: the
  `requests` row lists the new columns, and the new index is named.
- [x] `docs/rules/testing/the-viewer-is-framed-only-by-the-editor-under-a-token-path.md:1`: the
  embed prefix, why a cookie cannot work in a frame, the two `frame-ancestors` values, the three
  messages and who checks each, `nvs debug link`. Its Declined list is the one above.
- [x] `docs/rules/ide/the-debug-viewer-opens-from-one-button.md:1`: the two commands, the setting,
  the button's place, the panel as a frame, `nvs/debugViewer`, the start offer, the stop rule.
- [x] `docs/rules/ide/a-lens-shows-what-a-route-a-dump-and-a-throw-recorded.md:1`: the three lenses,
  their keys, the newer-file rule. `docs/rules/ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site.md`:
  its Declined entry on picking the request in the viewer is rewritten, because the panel now takes
  `go`.
- [x] `docs/rules/ide/the-extension-builds-no-ui-the-editor-already-has.md:17`: **Built** names the
  panel and why it is a frame.
- [x] `bun nv rules --render`, then `bun nv rules --check` is green.

### 2. the viewer has the website's palette in the system's fonts, a frame with a rail and a command palette, and one set of shared components

**Split:** none. The session writes the tokens, the frame and `viewer/src/ui/` itself, because every
later stage builds on their names.

- [x] `viewer/src/styles/tokens.css:1`: the palette block, the roles in both themes, the P3 copy,
  the scales for space, radius and motion from `website/src/styles/custom.css:74`. `base.css` and
  `ui.css` beside it. `viewer/src/style.css` is deleted once every rule in it has moved or gone.
  - [x] the tokens, with each role a `light-dark()` pair and the P3 copy
  - [x] the view rules split into `viewer/src/styles/<view>.css` on the new role names
  - [x] `style.css` deleted, `main.tsx` imports the layers
- [x] `viewer/src/styles/tokens.css:1` and `base.css:1`: the two font stacks and the type scale of
  the standing decision, with tabular figures and no ligatures in code. No rule outside
  `tokens.css` names a face.
- [x] `tools/nv/cmd/viewer.ts`: `--check` tests that the seven palette values equal the website's,
  and that the built stylesheet has no `@font-face`.
  - [x] `paletteDrift` and `hasFontFace`, with unit tests in `tools/nv/test/viewer.test.ts`
- [x] `viewer/src/ui/`: `Icon`, `Chip`, `Method`, `Status`, `Tabs`, `Dialog`, `Toast`, `Empty`,
  `Tile`, `Spark`, `Meter`, `Site`, each with its styles in `ui.css`.
- [x] `viewer/src/keys.ts:1`: the key map views register with, and the `?` dialog prints it.
  `viewer/src/app.tsx:60` binds through it, and `t` no longer changes the theme.
- [x] `viewer/src/app.tsx:90`: the frame of the drawing: the top bar, the rail, the live pill (its
  state comes in stage 7, and it shows *Live* until then) and the theme button.
- [x] `viewer/src/ui/palette.tsx:1`: the command palette on `Ctrl+K`, a dialog with a listbox, fed
  by the views, the saved filters and the actions a view registers.
- [x] `NV_BROWSER=1 bun test viewer/test/frame.browser.test.ts` is green, with one new case: the
  rail's current item has `aria-current="page"`. The check `viewer-built` is green.
  - [x] three new cases: the rail's `aria-current`, the palette on `Ctrl+K`, and the key list
  - [x] `requests` and `hostile` browser tests updated for the theme button and the icons

### 3. every list view is the shared list, and every view wears the new design

**Split:** after the session lands `viewer/src/ui/list.tsx` and moves `requests.tsx` onto it, four
writing subagents run side by side, none of them builds: `stream.tsx` with `log.tsx`,
`exceptions.tsx`, `search.tsx`, and `compare.tsx` with `replay.tsx`, `to-test.tsx` and
`coverage.tsx`. Each gets the list's signature, `ui.css`, the drawing and its view's browser test.

- [x] `viewer/src/ui/list.tsx:1`: the one virtual list: fixed row height, selection, `j`, `k`,
  `Home`, `End`, the page keys, `Enter`, follow and pause with the arrived count. It keeps the
  WAI-ARIA listbox roles.
  - [x] `List`: virtual rows, selection by key, the shared keys and a view's own through `keys`
  - [x] `createFeed`: the newest page, follow, pause, the arrived count and older pages
  - [x] the list's styles in `ui.css`
- [x] `viewer/src/views/requests.tsx:230`: the list and its keys come from `List`. The columns are
  the drawing's: method chip, path with the route beside it, status, duration, queries, age.
  - [x] the view is `List` and `createFeed` with the drawing's columns and a 38 pixel row
  - [x] `requests.browser.test.ts` reads the new cells and is green
- [x] `viewer/src/views/stream.tsx:246` and `log.tsx`: on `List`, with a level chip and a kind dot
  in the kind's tone. `Feed` and `FilterForm` move to their own files.
  - [x] `views/feed.tsx` is `createFeed` and `List`, with `KindDot` and `LevelChip`
  - [x] `views/filter-form.tsx` holds `FilterForm`, and `stream.css` is on the tokens
- [x] `viewer/src/views/exceptions.tsx:110`: on `List`, with `Spark` for throws over time.
  - [x] both lists are `List`, and the chart is `Spark` with throws drawn as `bad`
- [x] `viewer/src/views/search.tsx:150`: grouped results in the new design, on the shared keys.
  - [x] each group is a card with `Method`, path, `Status`, duration and age
- [x] `viewer/src/views/compare.tsx:245`, `replay.tsx`, `to-test.tsx`, `coverage.tsx`: tables,
  dialogs and the source view in the new design, with `Dialog` and `Toast`.
  - [x] `Dialog` takes `describedby` and carries `role` and `aria-modal`
  - [x] the toast region is a manual popover, so it draws above an open dialog
- [x] Each of `exceptions`, `hostile`, `replay`, `request`, `requests` and `stream` under
  `viewer/test/*.browser.test.ts` is run by name and is green.
  - [x] all seven browser tests with `frame`: 51 pass, 0 fail

### 4. the index holds each request's time by kind and its repeated statements, and the API filters and summarises

**Split:** none, one session. It shares no file with stages 2 and 3.

- [x] `crates/nvs-cli/src/debug/index.rs:78`: the eight columns, their trigger beside the one at
  `:318`, the index on `(method, route, start_us)`, and the schema version raised.
- [x] `crates/nvs-cli/src/debug/api.rs:319`: the filters of `GET /api/requests`, each a bound
  parameter.
- [x] `crates/nvs-cli/src/debug/summary.rs:1`: `/api/summary`, `/api/routes`, `/api/statements` and
  `/api/health`, with the bounds of the table, and their rows in the route table
  (`crates/nvs-cli/src/debug/listen.rs:326`).
- [x] `crates/nvs-lsp/src/recorded.rs`: an `Explore` subagent first reports whether the lens reads a
  column this stage changes. The lens keeps working against the new schema version.
- [x] `crates/nvs-cli/tests/debug_api.rs`: a test per filter and per new route, and the fixtures
  `viewer/test/fixtures/{summary,routes,statements,health}.json`. `viewer/src/api.ts:84` gains their
  types and the eight event kinds, and `viewer/test/api.test.ts` checks them. The check `api` is
  green.

### 5. the request list has filters, summary tiles, a time split per row and a peek pane

**Split:** none, one session.

- [x] `viewer/src/views/requests.tsx`: the filter chips of the drawing, each a popover of choices,
  and the filter is the view's query, so a filtered list has a link and can be saved.
- [x] The four tiles from `/api/summary`, refreshed when the watermark moves and never more than
  once a second. The memory tile is absent until a record carries a peak.
- [x] The duration cell: the figure and one bar split by kind, scaled to the slowest row on screen.
  The queries cell shows the gold badge when `repeats` is two or more.
- [x] The peek pane at 1280 pixels and wider: the selected request's tiles and a small timeline.
  `Enter` opens the page. Below that width there is no pane.
- [x] `Empty` for a list with no record: how to turn recording on, with the `nvs.toml` lines.
- [x] `viewer/test/requests.browser.test.ts`: cases for a filter in the link, the badge and the
  paused tiles, run by name.

### 6. the request page has a header, a time split and tabs for its timeline, request, response, queries, dumps, logs and coverage

**Split:** after the session lands the header and `Tabs`, two writing subagents run side by side:
the timeline (`timeline.tsx`), and the Request, Response and Queries tabs (new files under
`viewer/src/views/request/`).

- [x] `viewer/src/views/request.tsx:134`: the header of the drawing: method, path, status, the
  route, the age, the actions with *Replay* as the primary one, and *Copy as curl*.
- [x] The tiles and the time split bar with its legend, from the row's eight columns.
- [x] `viewer/src/views/timeline.tsx:46`: a waterfall with one row per event or per group of a
  repeated statement, all eight kinds in their tones, the axis, and the listbox kept for the keys.
- [x] `viewer/src/views/request/input.tsx:1` and `output.tsx:1`: headers and body from the recorded
  `input` and `response`, a JSON body through the value tree, and the sentences the compare view
  already has for a body that was not read, was cut or was withheld.
- [x] `viewer/src/views/request/queries.tsx:1`: statements grouped by call site, with count, total
  time, rows and the site link.
- [x] The Dumps, Logs, Coverage and Exception tabs hold what the page's sections held.
- [x] `viewer/test/request.browser.test.ts` and `hostile.browser.test.ts`: cases for the tab in the
  link, a hostile body staying text in both body tabs, and a repeated group, run by name.

### 7. the Routes view, the Queries view, the lost-records state and the settings page

**Split:** two writing subagents side by side after the session adds the routes to `router.ts`:
`views/routes.tsx` and `views/statements.tsx`.

- [ ] `viewer/src/router.ts:66`: `/routes`, `/queries` and `/settings`.
- [ ] `viewer/src/views/routes.tsx:1`: one row per method and route with count, error share, p50,
  p95, mean queries and last seen, sorted by a clicked column. `Enter` opens the list filtered to
  the route.
- [ ] `viewer/src/views/statements.tsx:1`: one row per call site with the statement, count, total
  and max time and how many requests repeated it. `Enter` opens the search for that statement.
- [ ] The live pill reads `/api/health`: its three states, and a click on the lost count opens the
  list of what was lost, moved out of `viewer/src/views/stream.tsx:508`.
- [ ] `viewer/src/views/settings.tsx:1`: the theme, the editor and its path mappings
  (`viewer/src/views/editor.tsx`), the saved filters and *Discard the index*, as one page. The
  dialogs that held them are gone.
- [ ] `viewer/test/routes.browser.test.ts:1` and `settings.browser.test.ts:1`, run by name.

### 8. nvs debug link prints the link, and the page is served under a token path that only the editor may frame

**Split:** none, one session. Read `docs/agent/coding.md` first: the stage changes a listener.

- [ ] `crates/nvs-cli/src/debug.rs:140`: the `link` subcommand with `--json`, its help, and its
  reference card.
- [ ] `crates/nvs-cli/src/debug/listen.rs:271`: the `/embed/<token>/` prefix: the constant-time
  compare before any route, no cookie, the page, the bundle and every `/api/` route
  under it, and `frame-ancestors vscode-webview:` on its responses alone (`:84`).
- [ ] `viewer/src/router.ts:9` and `viewer/src/api.ts`: the base path in front of every path, fetch
  and the event stream.
- [ ] `viewer/src/embed.ts:1`: the three messages and their checks. `viewer/src/ui/site.tsx` sends
  `open` when embedded.
- [ ] `crates/nvs-cli/tests/debug_viewer.rs`: a wrong token under the prefix is `401`, a right one
  serves the page and one API route with no `Set-Cookie`, the two `frame-ancestors` values, and
  `nvs debug link --json` against a running service. The check `embed` is green.
- [ ] `viewer/test/embed.browser.test.ts:1`: a page under a base path moves, fetches and sends
  `open`, and ignores a message from any other origin. Run by name.

### 9. the extension opens the viewer in a panel or the browser from one button, and a site in the panel opens in the editor

**Split:** none, one session.

- [ ] `crates/nvs-lsp/src/server.rs:2432`: the request `nvs/debugViewer`, its entry in the closed
  request set, and its case in `crates/nvs-lsp/tests/handshake.rs`.
- [ ] `editors/vscode/package.json:234`: the two commands, the setting, the `editor/title` entry,
  the `debug` command of the `nvs` task type, and their rows in
  `editors/vscode/test/contributions/contributions.test.ts`.
- [ ] `editors/vscode/src/debugviewer.ts:1`: the commands, the panel, the start offer and the
  message for `store` off. `editors/vscode/src/debugpage.ts:1` builds the panel's page and its
  policy, pure, as `viewer.ts` does for the profile.
- [ ] `editors/vscode/src/extension.ts:88`: `nvs.openDebugViewer` is server-callable, and the query
  and copy lenses call it with the request's path (`crates/nvs-lsp/src/recorded.rs:325`).
- [ ] Headless tests under `editors/vscode/test/debugviewer/`: the page's policy admits one frame
  source, an `open` for a file outside every workspace folder opens nothing, and a `go` path is
  checked. The check `extension` is green.
- [ ] A host test under `editors/vscode/test/host/`: the command opens a panel of type
  `nvs.debugViewer` against a service the test starts. It is the test the stop rule reads.

### 10. a lens shows what a route, a dump and a throw site recorded, and opens it in the viewer

**Split:** one writing subagent writes the `.lspt` cases under `tests/lsp/` once the session has
landed the three lenses.

- [ ] `crates/nvs-lsp/src/recorded.rs`: three bounded reads: a route's count, p50 and newest failing
  status, the newest dump at a file and line, and the exception group at a file and line.
- [ ] `crates/nvs-lsp/src/server.rs:2141`: the three lenses beside `query_lenses` (`:2376`), the
  newer-file rule, and their rows in `crates/nvs-lsp/src/lens_rows.rs`.
- [ ] Rust tests beside the query lens's: each lens against a fixture index, no lens with no index,
  and no dump lens for a file newer than its record.

### 11. the feature proofs, the screenshots, the help and the website show the new viewer

**Split:** one writing subagent reshoots and describes the screenshots, and one rewrites the two
pages, after the session has listed what each page must say.

- [ ] `website/snippets/tools/debug-viewer/`: every `.shot` is shot again with `bun nv site --shoot`
  in the dark theme, and four are new: `routes.shot`, `queries.shot`, `request-tabs.shot` and
  `lost.shot`. Each `alt` says what the picture shows now.
- [ ] `website/src/content/docs/tools/debug-viewer/index.mdx`: rewritten for the frame, the filters,
  the tabs, the two new views, the keys and `nvs debug link`, then stamped.
- [ ] `website/src/content/docs/tools/editor/index.mdx`: the button, the two commands, the setting
  and the three lenses, then stamped.
- [ ] The help of `nvs debug` and `nvs debug link`, and their reference pages.
- [ ] `website/src/data/changes.json`: `changed` the debug viewer, `new` `nvs debug link`, `new`
  the extension's *Open Debug Viewer*, `new` the route, dump and throw lenses.
- [ ] `viewer/design/reference.html` is deleted, and no file names it.
- [ ] `bun nv proofs --owed --goal debug-viewer-flagship` names nothing, and the checks `proofs` and
  `shots` are green.
- [ ] `bun nv site --check stale` and `changes` are green.

### 12. the goal is verified once, and every failure is fixed

- [ ] The newcomer review, [prose.md](../agent/prose.md) § *A newcomer reads it*, over every page,
  help text and card this goal wrote, and `bun nv site --check review` is green.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 6 are done. Four rules are `designed` and owned by this goal, and two are `shipped`:
`testing/the-viewer-wears-the-websites-palette` and
`testing/the-viewer-answers-per-route-and-per-statement`. The stage that ships a rule sets its
record `data/rules/<topic>/<slug>.json` to `shipped`, drops `goal` and fills `guardedBy`. The
`path:line` anchors in later stages come from surveys, so a stage checks an anchor before it edits.
The names stages 3 to 6 gave are under Standing decisions.

Stage 6 is the commit after `2208290491`. The request page has the drawing's header with
*Copy as curl* and *Replay* as the primary action, four tiles, the time split with its legend, and
tabs in the link: a waterfall Timeline, Request, Response, Queries, Dumps, Logs, Coverage and
Exception. The browser tests `request` (8 cases), `hostile`, `exceptions`, `requests` (13),
`stream`, `replay`, `frame` and `search` are green, as are `tsc`, `bun nv viewer --check` and the
new unit test `viewer/test/curl.test.ts`. The exceptions and request-list tests now read the
tabbed page. The bundle is 151,509 bytes of JavaScript and 43,237 of CSS. The nightly failures
the sweep named ran at `53e94a9181`, before their fixes `1c3196b0e8` and `8ca2f09e1b`.

### Next

Stage 7 from its first item, at `viewer/src/router.ts:66` (check the anchor): `/routes`,
`/queries` and `/settings`, then the split's two subagents for `views/routes.tsx` and
`views/statements.tsx`. They read `routes()` and `statements()` in `viewer/src/api.ts` and draw
with `List` (`viewer/src/ui/list.tsx`). A route opens the filtered list with
`viewLink("requests", listQuery({ method, route }))`, and the statements view can reuse
`Repeat` and the gold wash of `.query-group.repeated` (`viewer/src/styles/request-tabs.css`).
Routes and Queries join the rail (`viewer/src/app.tsx`) with their views. `bun nv site --check
shots` stays red until stage 11 takes the screenshots again. No test is expected to go red.

### Backlog

- The memory figures, the handler link, the exception's stack and the response size in the drawing
  are the goal `deeper-debug-records`. This goal draws each only when a record has the field.
