---
milestone: post-parity
position: last
---
# Loop goal 187 — `Core\Html::later`: a page goes out at once, and its slow parts fill in as they are ready

A developer marks the slow parts of a page with `Core\Html::later`. Everything else stays as it is:
`echo`, `html` literals, partials, layouts. One switch on the route decides how the page is
delivered, and the code does not change between the two:

```nvs
<?= Layout::header($post->title) ?>
<main>
  <h1><?= $post->title ?></h1>
  <?= Core\Html::later(fn() => Comments::render($post->id),
                       {placeholder: html`<p>Loading comments…</p>`}) ?>
</main>
<aside>
  <?= Core\Html::later(fn() => Recommendations::forUser($user), {deadline: 2s}) ?>
</aside>
<?= Layout::footer() ?>
```

```nvs
#[Core\Route(path: "/post/{id}", method: Core\Http\Method::Get, slotted: true)]
```

- **A normal route** runs every `later` closure concurrently while the main script goes on, puts
  each one's output where its placeholder was written, and sends the page once all are done. The
  page waits for the slowest closure and not for the sum of them.
- **A slotted route** sends the page as soon as the main script ends, with each placeholder in
  place. Each closure's output follows in the same response the moment it is ready, and the
  browser puts it where its placeholder is. One request, no second request, no code of the
  developer's own in the browser.

A component written once with `later` works on both kinds of route. An app mixes them freely: one
route slotted, the next normal, the JSON API untouched. A request that never calls `later` and is
not slotted runs exactly as it does today, and pays nothing for this goal.

## Why here

The user's decision of 2026-10-03, after the question of what output buffering means for a client
that leaves. Every Novis response is buffered whole today (`Output::Capture`,
`crates/nvs-server/src/serve.rs:1785`; PHP's `flush` is dropped, `docs/spec/02-php-migration.md:862`),
so the first byte waits for the slowest query on the page. The only way to send early is
`Core\Response::stream`, which cannot be mixed with `echo` (`crates/nvs-types/tests/response.rs:88`)
and writes `tainted` text raw (`crates/nvs-stdlib/src/response.rs:596-611`). This goal gives the
early first byte to an ordinary page without either of those: the developer keeps `echo` and its
escaping, and the runtime owns every byte of the streaming markup.

The user wants this as slick as possible for developers, and sees it as a major advantage over other
web languages. Out-of-order streaming exists elsewhere only inside frameworks with a JavaScript
runtime on both ends. Here it is one `Core` method and one route field.

It sits after goal `disconnect-completes` because that goal decides what a response's writes do once
the client has gone, and a slotted response is a response that keeps writing after its head went
out. It carries `position: last` because the goals around it do.

## What the browser receives

A slotted response is ordinary chunked HTML. The placeholder is the HTML standard's processing
instruction pair, and each fill is a `<template for>` (Declarative Partial Updates — the
[Chrome documentation](https://developer.chrome.com/docs/web-platform/declarative-partial-updates)
is the reference this goal was written against):

```html
<main>
  <h1>A post</h1>
  <?start name="nvs-Xq3vT9aLw2RkPz0b-1"><p>Loading comments…</p><?end>
</main>
…
<script>/* the polyfill, once */</script>
<template for="nvs-Xq3vT9aLw2RkPz0b-2">…recommendations…</template><script>_nvs()</script>
<template for="nvs-Xq3vT9aLw2RkPz0b-1">…comments…</template><script>_nvs()</script>
</body></html>
```

`<template for>` is in the HTML standard. Chrome 148 has it behind a flag and the other browsers do
not have it yet, so the runtime sends a small inline polyfill once per slotted response, before the
first fill. The server cannot know what the browser supports, so the polyfill is always sent. It
does nothing where the browser already did the work: it fills only a placeholder whose `?start`
marker and `<template for>` are both still in the page. Today's parsers read `<?start …>` as a
comment, so the polyfill finds the placeholders as comments. When every browser ships the feature,
the polyfill is removed and nobody's code changes.

**Why not declarative shadow DOM**, which works without JavaScript in every current browser: the
layout would live inside a shadow root, so the page's CSS does not reach it and
`document.getElementById` does not find it, and many JavaScript libraries break. The user chose
`<template for>` with the polyfill (2026-10-03). A browser with JavaScript turned off shows the
placeholders and never the fills, and the placeholder is therefore real fallback content and not
only a spinner, which the reference says.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong or incomplete, each rewritten whole by the session that
lands the behaviour and not before:

- `docs/rules/concurrency/nothing-is-still-running-when-a-call-returns.md` and
  `docs/reference/lang/80-concurrency.md:10-13` ("every task is a child of the call that started
  it"). A `later` task is a child of the **request**, not of the call: `later` returns at once, and
  the response does not end while a `later` task is running. The guarantee holds one level up.
- `docs/rules/concurrency/after-response-outlives-the-connection.md:9-11` and the module doc of
  `crates/nvs-runtime/src/deferred.rs:1-12` ("the request task's own frame returning … is the moment
  the response is fully written"). In a slotted response the frame returns before the last fill.
  The trigger becomes the response being complete: the frame has returned **and** every `later` task
  has ended.
- `docs/rules/core-classes/html-auto-escape.md` gains the placeholder: a `Core\Html\Markup` the
  runtime makes, whose bytes the developer never writes.
- `docs/spec/02-php-migration.md:862` (`flush` dropped, "a response is written by the runtime when
  the handler returns") gains `Core\Html::later` as what an early first byte is now.
- `crates/nvs-runtime/src/routes.rs:289` (`Route`) and its `nvs-types` twin gain `slotted`, and
  `#[Core\Route]`'s spelling in `crates/nvs-types/src/derive.rs:143` with them.

The stage closes with the same search it opens with:
`grep -rn "child of the call\|frame returning\|fully written\|flush" docs/rules docs/reference crates/nvs-runtime/src crates/nvs-stdlib/src/task.rs`,
read line by line. Every hit is either true as it stands, rewritten, or under `docs/decisions/`.
Generated files (`docs/novis.md`, `docs/ground-rules.md`, the `docs/rules/*.md` chapters, `website/`)
are regenerated, never edited.

## Stage 1 — the floor

Nothing is carried. Goal `goal-closeout` makes a finished goal deleted. The suites, the `.nvst` trees
and `nv verify` are the floor. Every response that never calls `later` keeps its bytes, its headers
and its timing exactly.

## Stage 2 — `later` on a normal route, the keystone

**Does:** Adds `Core\Html::later`, which runs a closure concurrently as a child of the request and
puts its output where its placeholder was written.

One file set: `crates/nvs-stdlib/src/html.rs`, `crates/nvs-stdlib/src/task.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-runtime/src/ctx/output.rs`,
`crates/nvs-runtime/src/deferred.rs`.

- **The decision record**, written first from § *Standing decisions*, for the whole goal. It
  `modifies` the three rules Stage 0 names and `adds` one rule for `later` under
  `docs/rules/core-classes/`, and the fragments are written with it.
- **The signature.**
  `Core\Html::later(callable(): void|Core\Html\Markup $fn, {placeholder?: Core\Html\Markup, error?: Core\Html\Markup, deadline?: Core\Time\Duration}): Core\Html\Markup`.
  The closure's output is what it echoes, followed by the `Markup` it returns, if any. `echo`
  inside it is the request's escaping sink exactly as everywhere else in the request
  (`rule:core-classes/html-auto-escape`), so a `tainted` string is escaped and a `Markup` is
  written raw. `placeholder` defaults to empty and `error` to a plain fragment the reference
  names. `deadline` has no default: a `later` without one is bounded by the request's own limits
  and nothing else.
- **The task is the request's child.** It starts at the call, runs concurrently with the main
  script and with every other `later` task, and is charged to the request tree like any task:
  memory, CPU, `max_tasks`. The response does not end while one is running. A `later` called inside
  a `later` closure is the request's child too, and its placeholder is part of its parent's output.
- **The placeholder.** `later` returns a `Core\Html\Markup` whose bytes are
  `<?start name="nvs-<token>-<n>">` + the placeholder + `<?end>`. `<token>` is 96 random bits made
  once per request, written as 16 base64url characters, so no author text and no visitor input can
  name a slot: a visitor's string is escaped before it reaches the sink, and an author cannot know
  the token. 96 bits cannot be guessed, and the name appears twice per slot on the wire, so it is
  kept short. `<n>` counts the
  request's `later` calls from 1. The runtime finds a placeholder in the buffered body by that
  token, so `Core\Html\Markup` keeps its one slot (`CARRIER_TEXT_SLOT`,
  `crates/nvs-runtime/src/ctx/output.rs:51`) and `+`, `Core\Html::join` and partials carry a
  placeholder like any other markup.
- **Assembly on a normal route.** When the main script and every `later` task have ended, each
  placeholder in the body, from `<?start` to `<?end>` inclusive, is replaced by its closure's
  output. One pass over the body. A filled closure's output may contain further placeholders, and
  they are replaced in the same pass.
- **A placeholder written nowhere.** A `later` whose placeholder is not in the body when the main
  script ends is cancelled then, and one `Warn` names the call's file and line. A placeholder
  written twice throws `LogicError` at the second write, because one slot has one place.
- **A slot's own failure.** A closure that throws, or passes its own `deadline`, fills its slot with
  `error`. The error is logged with the request, as an uncaught throw is. The status stays what the
  main script set, and the other slots go on. The same happens on a normal route and on a slotted
  one, so the two look alike.
- **The request's limits are the only limits.** There is no limit of `later`'s own. Every `later`
  task shares the request's `wall_time`, `cpu_time` and `memory` with the main script
  (`rule:security/isolate-budget-is-the-trees`), so a page with `later` is bounded exactly as the
  same page without it. When one of those limits is reached, the request fails as a whole, as it
  does today: on a normal route nothing has been sent, and the response is the one a limit breach
  gives now. On a slotted route the head and the shell have been sent, so every slot that has not
  been filled gets its `error` fragment, the held-back end of the document is written, and the
  response ends. `Core\Fatal::onLimit` sees the breach as it sees any other. A slot that never ends
  is the same case as a page that waits forever on a query: `wall_time` bounds both, and goal
  `disconnect-completes`'s grace bounds both once the client has gone.
- **A `later` closure cannot change the response head.** Inside one, `Core\Response::setStatus`,
  `setHeader`, `redirect`, any body method (`html`, `json`, `text`, `stream`, `sendFile`), a cookie
  write and `Core\Session::regenerate` throw `LogicError`, on both kinds of route. Where the
  checker can see the call inside the closure's own body it is a compile error as well, with a new
  diagnostic code. Reading the request and the session is allowed.
- **Not an HTML response.** `later` in a request whose body method is not HTML, or outside an HTTP
  request (a CLI program, a job, a test without a request), runs the closure in place and returns
  its output, so a component that uses it still works there.
- **After-response work** runs once the response is complete, after the last `later` task.
- **Pinned by** the Stage 2 checks and two `.nvst` cases: one prints a page with three `later` slots
  that finish in reverse order, a nested one, an error and a deadline, and shows the assembled body;
  one is the closure that sets a header and does not compile.

## Stage 3 — slotted delivery

**Does:** Sends a slotted route's page as soon as its main script ends, and each `later` output the
moment it is ready.

One file set: `crates/nvs-server/src/serve.rs`, `crates/nvs-runtime/src/stream.rs`,
`crates/nvs-runtime/src/routes.rs`, `crates/nvs-runtime/src/ctx/output.rs`,
`crates/nvs-types/src/derive.rs`, `crates/nvs-types/src/routes.rs`, `crates/nvs-stdlib/src/response.rs`.

- **The two switches.** `#[Core\Route(…, slotted: true)]` is the usual one: a new optional field,
  carried to the runtime's route table as `csrf` is (`crates/nvs-runtime/src/routes.rs:289`), and
  read when the request matches its route, before the first line runs. `Core\Response::slotted()`
  is the one for a decision made while running, such as slotting only for a signed-in user. It must
  be called before the main script ends; afterwards, and inside a `later` closure, it throws
  `LogicError`. A request that never calls `later` on a slotted route is sent whole, as today.
- **The head and the shell go out when the main script ends.** Status, headers and cookies are all
  decided by then, which is why a `later` closure may not change them. The shell is the body as
  the main script left it, placeholders in place, sent as the first chunk. Nothing is sent before
  the main script ends: a page whose main script is slow sends late, and that is the developer's
  to move into a `later`.
- **Fills, in the order they finish.** Each fill is `<template for="nvs-<token>-<n>">` + the
  closure's output + `</template>` + the trigger `<script>_nvs()</script>`, written as one chunk.
  The polyfill `<script>` is written once per response, just before the first fill, and never on a
  normal route or on a slotted response with no fill.
- **The end of the document.** If the shell ends in `</body>` and `</html>` (case-insensitive,
  whitespace allowed), those bytes are held back and written after the last fill, so every fill is
  inside `<body>`. A shell that does not end that way gets its fills after its last byte, where
  every HTML parser still puts them in `<body>`.
- **The polyfill** is Novis's own, written for this goal and kept as a minified constant beside its
  `sha256`. No third-party script is shipped. It defines one function, `_nvs()`, which fills every
  placeholder whose `<template for>` has arrived, and it runs `_nvs()` once when it loads.
  - **It acts only where the browser did not.** It fills a placeholder only when its `?start`
    marker and its `<template for>` are both still in the page, and it removes both when it has
    filled it. A browser that applied the fill while parsing leaves nothing for it to do, so it
    never needs to ask the browser whether it supports the feature, never fills a slot twice, and
    also covers a browser that supports only part of it. Whether a native browser removes the
    template, the marker or both is not documented, which is why the polyfill tests both.
  - **No MutationObserver.** Each fill is followed by the same trigger, `<script>_nvs()</script>`,
    so a fill appears as soon as its chunk is parsed and not after an observer's delay. The trigger
    is identical every time, so one hash covers it as one covers the polyfill.
  - **The CSP.** When the response carries a `Content-Security-Policy` with `script-src` (or a
    `default-src` that covers scripts), the runtime adds the two `'sha256-…'` values, the
    polyfill's and the trigger's, to it. With the default policy, which sets only
    `frame-ancestors` (`crates/nvs-config/src/default.toml:317-321`), nothing is added.
- **Size limits, enforced by tests.** The minified polyfill is at most 1 KB. The bytes a slot adds
  on the wire beyond its own content and placeholder (the `?start` and `?end` markers, the
  `<template for>` wrapper and the trigger) are at most 120. A change that goes over either fails
  the test, and the limit is raised only by the user.
- **Headers.** A slotted response is chunked and has no `Content-Length`. It carries
  `X-Accel-Buffering: no`, so nginx sends each fill as it comes and does not hold the response. A
  proxy that compresses or buffers on its own can still delay fills; the reference says so. Novis
  itself does not compress responses (`crates/nvs-server/src/serve.rs:8224`).
- **The admission place, the drain and the disconnect.** A slotted response holds its place until
  its last fill, as a `Streamed` response does (`serve.rs:995`). The drain waits for it like any
  in-flight response. When the client leaves, goal `disconnect-completes`'s rule applies: the
  `later` tasks run to their end and their fills are thrown away, or, for a method in
  `cancel_on_disconnect`, they are cancelled.
- **`HEAD`** on a slotted route sends the head and no body, and the `later` tasks still run, as the
  main script does for any `HEAD`.
- **Pinned by** the Stage 3 checks: the first chunk arrives while a `later` task is still parked;
  fills arrive in finishing order; the polyfill is sent once and only on a response with a fill;
  every fill ends with the trigger; `</body></html>` is held back; the two CSP hashes are added
  only to a policy that limits scripts; the polyfill and the per-slot overhead stay inside their
  size limits; a
  `later` that sets a header still throws; `Core\Response::slotted()` after the main script throws.
- **The polyfill's own test** runs it against a DOM: a page where the fills are still templates
  gets every slot filled once and both markers removed; a page where the browser already applied
  the fills (no template, or no marker, left) is not changed at all; a fill whose template arrives
  before the polyfill and one that arrives after it are both filled; calling `_nvs()` twice changes
  nothing the second time. The repository has no DOM library and no browser test today, so the
  session adds `happy-dom` as a dev dependency in the root `package.json` for this test alone, and
  writes it as
  `tools/nv/test/html-later-polyfill.test.ts` reading the constant from the Rust source.

## Stage 4 — tests, the editor and the help

**Does:** Makes `later` work in `Core\Test`, in the editor and in `nvs help`.

One file set: `crates/nvs-stdlib/src/test.rs`, `crates/nvs-lsp/src/completion.rs`,
`crates/nvs-lsp/src/hover.rs`.

- **`Core\Test`** requests return the assembled page, slotted or not, so a test asserts on the page
  and does not care how it was delivered. A slotted request's test response also has
  `slotted(): bool`, and the order the fills were sent in for a test that wants it, named in the
  record and nowhere else.
- **The editor.** Completion offers `later`'s three options with their types. Hover on `later` and
  on `slotted:` shows the reference card. The new diagnostic for a head change inside a `later`
  closure has its quick-fix text saying where to move the call.
- **The help.** `nvs help Core\Html::later` and the `#[Core\Route]` card's `slotted` field.

## Stage 5 — the feature proofs and the reference

**Does:** Adds the reference, the tests, examples, attack and bench for `later` and slotted routes.

- **The reference.** `Core\Html::later`'s card in `crates/nvs-stdlib/src/html.rs`, and a new section
  in `docs/reference/tools/25-server.md`, `# Pages that fill in: slotted routes`, with the wire
  format above, the polyfill, the proxy note and what a browser without JavaScript shows.
  `docs/reference/lang/80-concurrency.md` gains a short section on a task whose parent is the
  request. Both are new headings, so new features on the roster, and `bun nv proofs --id` names what
  each owes.
- **Examples** under `docs/examples/core/Html/later/`: a page with two slow parts on a normal route;
  the same page slotted; a component with `placeholder`, `error` and `deadline`. Under the server
  section's folder, each with its `nvs.toml`: one app with a slotted route beside a normal one and a
  JSON route; `Core\Response::slotted()` for signed-in users only.
- **The attack** under `tests/hostile/core/Html/later/`: a visitor's input that copies the
  placeholder syntax with a guessed name is escaped and fills nothing; a `later` closure tries to
  set a header, a cookie and a redirect, and each throws; a request starts `later` calls in a loop
  until its `memory` limit stops it; a slotted page whose `later` never ends reaches its
  `wall_time`, and every unfilled slot shows `error` and the response ends.
- **The bench** under `benches/members/`: a page with three `later` slots of 50 ms each, on a normal
  route and slotted. It measures the counts of the assembly pass and of the fill writes; the time to
  the first byte and to the last is reported beside them, never as the headline.
- **The help** in the binary, and the `about.md` of each new feature.
- `bun nv reference` regenerates `docs/novis.md`.

## Standing decisions

- **The user's calls, 2026-10-03.** The method is `Core\Html::later`. The markup is
  `<template for>` with an inline polyfill, not declarative shadow DOM. A route turns slotting on
  with `#[Core\Route(…, slotted: true)]`, and `Core\Response::slotted()` covers a runtime decision.
  A failed slot shows its `error` fragment and the page keeps its status, on both kinds of route.
  On a normal route the `later` closures run concurrently. An app mixes slotted and normal routes
  freely, and the code of a component does not depend on which one includes it. **`later` adds no
  limit of its own**: it shares the request's limits, and a limit breach fails every unfilled slot,
  as the same page without slotting would fail. **The polyfill** is sent once per slotted response
  with a fill, acts only where the browser did not, uses the identical `_nvs()` trigger after each
  fill and no MutationObserver, and stays within two size limits a test enforces: 1 KB for the
  polyfill, 120 bytes of overhead per slot. Slot names carry a 96-bit token in 16 characters.
- **My calls, not yet confirmed by the user:** the option names `placeholder`, `error` and
  `deadline`; the closure returning `void` or `Markup`; `happy-dom` as a dev dependency for the
  polyfill's test;
  cancelling a `later` whose placeholder is never written; `LogicError` for a placeholder written
  twice; the optional per-call `deadline` with no default; Novis's own polyfill and its CSP
  hashes; holding back `</body></html>`; nothing sent before the main script ends; after-response
  work running after the last fill. A session that finds one of these impossible writes it under
  the handoff's `## Backlog` for the user rather than choosing again.
- **Not in this goal:** flushing the shell before the main script ends; a `later` in `<head>`
  (a `<title>` cannot be a slot, because the head is sent first); declarative shadow DOM as a
  second markup; a fill that replaces content outside its own placeholder. Each is a possible
  later goal and goes to the handoff's `## Backlog` if a session meets it.
- **Simplicity of the surface is the point.** One method with three options, one route field, one
  call. A session that finds itself adding a fourth thing a developer must know writes it under
  `## Backlog` for the user instead.
- **No new carrier slot.** `Core\Html\Markup` keeps exactly one slot; the token is what makes a
  placeholder findable.
- **Two ADR slots**: the record for `later` and slotted routes, and, only if Stage 3 needs it, one
  for the polyfill and its CSP hash. No other number, each checked right before it is written. The
  first states the tradeoffs. Performance: no cost for a request that never calls `later`; one pass
  over the body to assemble; on a slotted route the first byte goes out when the main script ends,
  and the last when the slowest slot ends. Memory: each `later` task's output is held until it is
  sent, per request and under its memory cap, and the shell is held from the end of the main script
  until it is written. Usability: an early first byte with no change to how a page is written.
  Simplicity: one method, one route field, one call; the developer never writes streaming markup,
  and a browser without JavaScript shows the placeholders.
- **Every name in a test, an example and the record is neutral** — `Blog`, `Shop`, `example.com`.
- **Every comment in a new `.nvs` and every new `about.md` follows `AGENTS.md` § *Text an end user
  reads* at the first write**, and `bun nv proofs --comments <paths>` is run over them before the
  wrap.
