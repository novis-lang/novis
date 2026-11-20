# The comparison: Novis and its peers behind one TLS proxy

`bun nv bench-compare` runs this directory. It measures servers end to end — `nvs serve`, PHP-FPM,
`Bun.serve`, Go's `net/http`, Node.js's `node:http`, ASP.NET Core on Kestrel and a Python ASGI program under `uvicorn` —
each behind the same nginx terminating TLS
1.3, on the same CPU budget, with the same generator. Its record is `benches/compare.json`, and the website's comparison
page is rendered from it.

**This file is the only home for why the comparison is built the way it is.** What it shares with the
proxied leg — nginx in front, one stack up at a time, the CPU budget and its two arms, `oha` inside the
stack's network, tuning in git with a digest in every row — is decided in
[benches/proxied/README.md](../proxied/README.md), and nothing here restates it. How a run is carried out
is `tools/nv/cmd/bench-compare.ts`'s header.

## Why these peers

Novis against the runtimes a team choosing a web server is likely to weigh it against. PHP, because
FPM is the deployment Novis replaces. Bun and Node.js, as the JavaScript peers: Node.js is the runtime
most JavaScript servers run on, and Bun the one that competes with it on speed, so a reader sees both.
Go, as the compiled peer with a standard-library server. .NET, as the JIT-compiled peer whose server,
Kestrel, ships with the runtime. Python, as the scripting language most web back ends are written in
after PHP and JavaScript. Python is measured as `rule:tooling/python-claims` permits: a measured figure,
quoted with its engine, mode and host. That rule keeps "faster than Python" and "replaces Python" off the
page, and requires the page to say that Novis has no REPL and no numeric or machine-learning stack.

Each peer but Python uses its standard library only, and every peer is tuned to its own advantage with
every setting in git:
FPM with a static pool, opcache and the JIT; Bun with one process per CPU of the budget sharing the
port with `reusePort`; Node.js with a `node:cluster` primary forking one worker per CPU of the budget,
the cluster documentation's own example, and a keep-alive timeout longer than nginx's, as its
documentation advises behind a proxy; Go with `GOMAXPROCS` set to the budget, because the Go runtime
reads the host's cores and not the container's quota; .NET as a framework-dependent Release build at
its defaults, which are server GC and tiered compilation with dynamic PGO, with the logging
`dotnet new web` writes (`dotnet/appsettings.json`), so ASP.NET Core logs no line per request. Native
AOT is not used: the JIT with PGO is .NET's default deployment, and the faster one for a long-running
server.

Python's standard library has no production server: the `http.server` documentation says it is not
for production use. So Python runs an ASGI program with no framework under `uvicorn`, the server
FastAPI's documentation deploys with. It holds the place PHP-FPM holds for PHP, and it is the one piece
of the comparison outside a peer's standard library. `uvicorn` runs with `--workers` equal to the
budget's CPUs, the event loop `uvloop` and the HTTP parser `httptools` its documentation recommends, no
access log, and a keep-alive timeout longer than nginx's, as Node.js's has. The three are pinned in
`python/requirements.txt`, and `python/Dockerfile` installs their published wheels and holds every
flag. CPython runs as its standard build, which runs one thread at a time.

`nvs serve` runs with its defaults. The one exception is PHP's file
check, which stays on at PHP's defaults (`php/file-check.ini`). The page groups PHP with Novis as a
server whose copied files are live, and with the check off a copied file is not live until a reload.

## Why TLS, and why at nginx

Every peer is deployed behind a proxy that terminates TLS, and that is the path a browser's request
takes. Novis has no TLS listener at all (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`),
so a comparison without the proxy would measure a deployment Novis does not have. The proxy's cost is
in every number, and it is the same cost for every peer: one nginx config for the HTTP backends
and one for FastCGI, kept line-for-line parallel so a diff shows the backend and nothing else.

TLS 1.3 only, HTTP/1.1, keep-alive on both sides of the proxy. The certificate is self-signed, made
once by `alpine/openssl` into the git-ignored `generated/tls/`, and `oha` runs with `--insecure`.

## One compose file per peer, and a common one

Each peer has its own `compose.<peer>.yaml`, for the proxied leg's reason: two stacks up at once is a
deliberate act. nginx and `oha` are declared once in `compose.common.yaml` and taken with `extends`, so
the proxy and the generator cannot drift between peers. Each peer publishes nginx on its own loopback
port, so a stack left running names itself.

The peers are data. `PEERS` in `tools/nv/cmd/bench-compare.ts` names each one's compose file, nginx
config, port, image and the settings written into its rows. A new peer is a new entry and a compose
file, never a new code path.

## One program per case per peer

A case is a folder under `cases/` with one program per peer: `main.nvs`, `index.php`, `server.ts`,
`main.go`, `server.mjs`, `Program.cs` and `app.py`. The stack mounts or builds the case it is brought up for, so no peer
carries a router the others do not. Go compiles every case into its image once (`go/Dockerfile`), and
.NET builds each into its image with one shared project file (`dotnet/Dockerfile`, `dotnet/App.csproj`). Bun
runs the case's `default { fetch }` under `bun/serve.ts`, and Node.js runs the case's default request
listener under `node/serve.mjs`. `uvicorn` imports the case's `app` from the mounted folder.

`json-api` is built so every peer's answers are byte-equal. Money is integer cents, and the 20 percent tax
is rounded half up by integer division, so no peer's float formatting shows. Keys are written in one
fixed order. The items are sorted by a stable sort, largest total first, and `order.json` has two items
with one total, so a peer whose sort is not stable disagrees. Every failed check answers 422 with one
fixed body. Each peer checks types in its own language's usual way: Novis, Go and .NET decode into typed
classes, PHP, Bun, Node.js and Python test the decoded value. The rules are the same set, never the same code.
Python's `json.loads` reads `NaN` and `Infinity`, which JSON does not have, so its program turns them away.

`html-page` builds its 100 products again on every request and escapes each name and the search term
the way each peer's own documentation says to: an html template in Novis, `htmlspecialchars` in PHP,
`Bun.escapeHTML` in Bun, `html/template` in Go, `html.escape` in Python and `HtmlEncoder` in .NET, created with
`UnicodeRanges.All` so a letter such as `é` is not encoded. Node.js has no HTML escaper in its standard
library, so its program replaces the five characters in a function of its own. The timed term holds `<`, `>`, `&`, `"` and `'` and
no `+`, because `html/template` alone also escapes `+` and that difference is not one the comparison
maps away. Prices are integer cents formatted with integer division, for `json-api`'s reason.
`html/template` resolves each value's type by reflection as it executes, and `bun nv bench-compare
--direct` measured about 245 µs of CPU per page for Go against 35 µs for Bun and 56 µs for Novis; the
Go program keeps it because it is the package Go's documentation names for HTML, and a hand-written
escaper would be the comparison choosing how Go is written.

`wait` sleeps 50 ms on each peer's own timer at 1000 connections, so its memory figure is what 1000
requests in flight cost. The FPM pool is `phpChildren(1000)`, 2000 workers, because a worker is held
for the whole sleep and a pool smaller than the open connections stops rather than slows (the proxied
leg's `php/pool.conf.in`). `nvs serve` keeps its defaults here as everywhere: it admits a request
while the memory the running requests hold leaves room for one more to reach its cap, and a request
asleep on a timer holds a few kilobytes, so the 1000 waiting requests are admitted while the
container has room for them and one cap. A run whose `wait` row shows a `503` in its status codes is
measuring admission and not the timer; `nvs` reads `nvs.toml` from its working directory, not from
the program's.

## The feature scenarios

A row is a folder under `scenarios/` with the same programs a case has, and a probe in
`SCENARIOS` that sends it requests. The stacks serve a row exactly as they serve a case: the compose
files mount `${TREE}/${CASE}`, and Go builds both trees into its image. A row that is about a case's
own answer, as `headers` is about hello's, points at that case and has no folder.

**A result is read off what the peer did, never written down.** Each probe maps its evidence to one of
five results with a pure function, and the evidence goes beside it: a page that says `no` can show the
request that proved it. A row whose Novis result is worse than the goal's facts expect is reported,
never adjusted.

**A feature that works through a setting names it, and says whether the peer starts with it on.**
`built-in` has no setting. `on-by-default` names a setting the peer ships turned on, which a reader
could turn off: Novis's `[http.headers]`, PHP's `max_execution_time` and `memory_limit`. `one-setting`
names a setting the row had to change, or the command or step the peer needed. The `Setting` tables in
`bench-compare.ts` say which is which.

**Each peer is written the way its own documentation shows.** The programs are what a developer meets
first, not the most careful version a peer allows. `escape` prints the term twice. On `/` it uses
PHP's `<?=`, a Bun template literal, Go's `html/template`, Node.js's escaper function, .NET's
`HtmlEncoder`, a Python f-string and a Novis html template. On `/direct` it uses the plainest write to
the response: PHP's `echo`, a Bun string joined with `+`, Go's `fmt.Fprintf`, Node.js's `res.end` of a
joined string, .NET's `WriteAsync` of a joined string, a Python string joined with `+` and a Novis
`echo` of strings. Python's standard library has no HTML template, so the f-string is the way its
documentation formats text. The row is `built-in` only when both arrive escaped,
because a safe way that sits beside an unsafe one is a choice the developer has to get right.
Otherwise the developer chooses the safe way or calls an escaper, so the result is `your-code` and
not `no`.

- `cores` and `waiting` send no requests of their own. Each times a request the way a full run does,
  on a fresh stack per pass, with the run's `--requests` and `--reps`.
- `cores` times one part of the `parallel` row's loop, about a millisecond of one core, at one cpu
  and at four. Throughput that grows by 2x or more is `built-in`, or `your-code` where the peer's
  program carries a launcher to use the cores, as Bun's `serve.ts` and Node.js's `serve.mjs` do.
  `uvicorn --workers` is the server's own flag, so Python carries no launcher. Less than 2x is `no`. A case
  in `cases/` cannot measure this: nginx and `oha` each have four cpus, and a peer that answers html-page
  at 20k requests a second on one cpu meets their ceiling at four, so its scaling would be the front's.
  Four cpus of a millisecond each answer a few thousand requests a second, far under that ceiling.
- Every backend has the restart policy `unless-stopped`, because every peer is deployed under a
  supervisor that starts a server again when it exits. A row reads the container's restart count, so a
  crash still shows, and the load after it measures the restart rather than a server left down.
- `crash` sends one request that throws or panics, then one that does not. The second must answer
  `200` from a backend that is running and was never restarted. A throw in a `node:http` listener
  ends the worker process, and Node.js's documentation says a process must exit after an uncaught
  exception. `node/serve.mjs` then ends the container, as `bun/serve.ts` does when a process exits,
  and the restart policy starts it again. A primary that forks a replacement would hide the crash
  from the restart count while the worker's other requests were still lost. ASP.NET Core catches an
  exception a handler throws and answers `500`, as Go's `net/http` recovers a panic, and so does
  `uvicorn` for an exception an ASGI program raises.
- `loop` and `memory` each put 15 seconds of steady load on `/` with nothing else running, then 15
  seconds more with one runaway request sent 3 seconds in. Five unmeasured seconds of the same load
  come first, so both loads start on a warm server: the first load on a fresh one pays for opcache,
  the JITs and new FPM workers, and PHP once answered more requests beside the runaway than without it.
  Each of the three runs is a fresh stack. In each run, `loop` sends `/loop`, which never ends,
  and `memory` sends `/alloc`, which allocates 1 MiB at a time without end. The first load is the
  peer's own throughput, and the page shows the share of it the second load kept, so a peer that is
  fast anyway does not win the row on speed. Each backend container has a 512 MiB cap in these rows, so
  an allocation without end stops at the container and not at the host. The runaway request counts
  as stopped when the backend answers it within 55 seconds, under nginx's 60-second read timeout,
  with any status but nginx's own `502` or `504`. No failed request under load, the runaway stopped
  and the backend never restarted is `built-in`. A request `oha` still has open when the load ends is
  not a failure. Every peer runs at its defaults. Novis's `[limits]` and PHP's `memory_limit` and
  `max_execution_time` both stop a request at 128 MB or 30 seconds of CPU time, so both are
  `on-by-default` naming the key their row needs. Thirty seconds is longer than the load, so `loop`
  measures the other requests while the runaway one still runs. Bun, Node.js, Go, .NET and Python have no limit for
  one request, so their answers are `no`, and the load's p99 stays in the evidence. Bun's and
  Node.js's `loop` is `your-code` (`LOOP_IN_CODE`): `Worker.terminate()` stops a worker that never
  yields, so a program that runs each request in a worker under a time limit stops it. Node.js's
  `memory` is `your-code` too (`MEMORY_IN_CODE`), because its `Worker` honours `resourceLimits`. Bun
  ignores a worker's `resourceLimits`, measured with `oven/bun:1.4.3-slim` and stated in its Node
  compatibility page, so its `memory` has no such way. .NET cannot stop a thread from outside:
  `Thread.Abort` throws `PlatformNotSupportedException`, and a `CancellationToken` stops only code that
  checks it. .NET's `memory` is `built-in`: in a container with a memory cap, the GC limits its heap
  to 75 percent of the cap, so the runaway allocation throws `OutOfMemoryException`, and ASP.NET Core
  answers that request `500` while the others go on. Python cannot stop a thread from outside either,
  and a coroutine that never awaits blocks its worker's event loop until it returns. Each of Go, .NET
  and Python could run a request in a child process and kill it, which is a second program and not a
  way the peer has, so none of them has a `your-code` entry. Python's runaway allocation passes the
  container's cap, and the container restarts, so its `memory` is `no`. No framework on either side stops a goroutine or a blocked event loop: Gin,
  Echo, Fiber, chi, Hono, Express and Fastify time out the reply and leave the handler running.
- `flood` puts load past capacity on the backend for 10 seconds: 200 connections send `/hold`
  without pause, and each hold keeps 8 MiB for 300 ms. That wants 1.6 GB, three times the 512 MiB
  cap the backend has in this row, as in `loop` and `memory`. Every 250 ms beside the load, the
  driver sends `/` and waits up to 10 seconds. A probe holds when it answers `200`, or `503` with
  `Retry-After`, which tells a client to come back. Once the load ends, `/` must answer `200` within
  20 seconds, and then 8 holds sent together must all answer `200`. Every probe held, the recovery,
  and a backend that never restarted is `built-in`, or `one-setting` naming PHP's
  `pm.max_children`, which this comparison writes for every PHP stack. Anything else is `no`. The
  evidence has the load's refusals (its `503`s), the probes' median and slowest time, and the seconds
  the recovery took. Every peer runs at its defaults. On one cpu a hold costs about 25 ms of CPU, so
  the load is past every peer's CPU as well as its memory. `nvs serve` admits a request while the
  memory running requests hold leaves room for one more to reach its cap. The holds of one burst all
  arrive before any of them has allocated, so Novis admits them, and those that grow past the room
  are stopped with `500`. Every CI run runs the row's `--smoke` form, which fails only when
  a stack cannot serve the row.
- No row tests whether a request sees a value another request set. A process that keeps state
  between requests is a design choice in Bun, Node.js, Go, .NET and Python, not a failure, and a yes or no cannot say which
  of the two a reader wants.
- `taint` and `secret` run `nvs check` over a program that puts a request value into an SQL string,
  and over one that prints a secret, in a one-off app container with no stack up. The check must
  fail with the row's own error code, so a program that fails for another reason is not counted.
  PHP, Bun, Go, Node.js, .NET and Python have no such check, so they run nothing. Their result is `no`, and the evidence
  links the page in their own documentation that warns about the mistake.
- `csrf` sends two `POST`s to a route that changes state: one with no `Origin`, which must change
  it, and one with `Origin: https://other.example`. The second is rejected when its reply does not
  say the state changed, whatever the status. Accepted is `your-code`, because a check of the
  `Origin` or a token is code or a library the developer adds.
- `session` starts a session, sends `/slow`, which writes the session and then works for two
  seconds, and 200 ms later `/fast` with the same cookie, which only reads it. `/fast` answered in
  under a second is `built-in`. Longer is PHP's session lock holding it until `/slow` ends, and
  `your-code` names the two calls that release it early. Bun, Go, Node.js and Python have no session without
  a library, so they run nothing and are `your-code`. ASP.NET Core's session middleware ships with
  .NET, so .NET runs the row with `AddSession` and `UseSession`. The row does not test lost writes: with no lock,
  two requests of one visitor that change one value keep the last write, which the page says.
- `headers` reads hello's response for `X-Content-Type-Options: nosniff`, a frame policy
  (`X-Frame-Options` or a `frame-ancestors` CSP) and a `Referrer-Policy`. Any one missing is
  `your-code`. All three from Novis are `on-by-default`, naming `[http.headers]`, whose shipped values
  send them.
- `waiting` times the wait case at one cpu. Every request answered `200` is `built-in`, or
  `one-setting` where the peer needs a setting before it holds 1000 connections: FPM's
  `pm.max_children`, whose default pool is a handful of workers. Any other answer is `no`. The memory
  per request is `memory.peak` less the idle `memory.current`, over the requests in flight at once,
  which is the concurrency or the run's `--requests` if that is smaller.
- `parallel` gives the backend four cpus whatever the run's budget. It sends `?parts=1` and then
  `?parts=4`, three times each, and keeps the fastest of each. One part is the same integer loop in
  every program, long enough that starting a thread or a child is a small share of it. With four
  parts, Go starts goroutines, Bun starts `Worker` threads, Node.js starts `worker_threads`, .NET
  starts `Task.Run` tasks on its thread pool, Python starts processes from a `ProcessPoolExecutor`,
  because its standard build runs one thread at a time, and
  Novis spawns child scripts
  `on: "worker"`. PHP-FPM has no way to do it, so its parts run one after another. A speedup of 2x
  or more over four parts in a row is `built-in`. Novis denies `spawn script` by default, so it
  writes `spawn = true` in its `[capabilities.script]` table first and is `one-setting`. Less than
  2x is `no`.
- `live-edit` serves a copy of its folder under `generated/`, changes `version 1` to `version 2` in
  the copy, and polls for 20 seconds. Each peer runs at its own defaults. PHP runs every case and
  row with `opcache.validate_timestamps=1` and `revalidate_freq=2`, its defaults, from
  `php/file-check.ini`, so an edit that shows is `on-by-default` naming the setting. Bun runs
  without `--hot`, Node.js without `--watch` and `uvicorn` without `--reload`, their defaults. An edit that never shows is `one-setting` where the peer's
  documentation names a setting it starts without, and the measured `null` stays in the evidence. Go
  and .NET compile their programs into the image, so they run nothing and are `no`.
- `deploy`, `deploy-big` and `deploy-switch` serve a program that reads part files, each
  printing its version, so a reply shows every part's version. `deploy/deploy.sh` writes the parts, and
  Novis's entry too, because `require` takes only a path written as literal text. It runs inside the
  container and writes to a Docker volume, because Docker Desktop's file share reports thousands of
  changed host files late, and a Linux server has no such share. Every peer's program but Novis's
  reads the part count from `DEPLOY_PARTS`. PHP runs at its default
  `opcache.validate_timestamps=1`, as in every case and row. The load is the driver's own: 8 loops of
  `GET /`, each reply checked, because `oha` never reads a body and a sampler beside it checks a few
  replies out of many. A reply is failed (an error, or any status but `200`), mixed (more than one
  version in it) or complete. Two seconds in, the driver writes version 2 one file at a time with a
  10 ms pause, then runs the step the peer needs at its defaults: none for Novis and PHP, which check
  their files, and a restart for Bun, Node.js, Go, .NET and Python. The load stops a second after the first reply of the
  new version, or 20 seconds after the step without one. Go and .NET compile their parts into the image, so they
  only restart, and the new version is the first reply to a request sent after the restart.
  `deploy` replaces 200 parts in place, about 2 seconds of copying, and `deploy-big` 2000, about 20.
  `deploy-switch` is the deploy PHP's own tools recommend: the folder holds `releases/1/` and a
  `current` link to it, the driver writes the whole of `releases/2/`, and one rename inside the app
  container switches the link. Every peer serves from `/app/current`. PHP's nginx uses `nginx/fastcgi-switch.conf`, which names the script under
  `$realpath_root`, as Deployer and Laravel Envoyer document, so one request reads one release. That is
  PHP at its best for a switch, which is the comparison's rule. A mixed reply depends on where PHP's
  2-second file check lands in the copy, so one copy proves little: each row runs three times on a
  fresh stack, sums the replies, and keeps the slowest seconds. No failed request, no mixed reply and
  the new version within 20 seconds of the step in every run is `built-in` with no step,
  `on-by-default` naming PHP's `opcache.validate_timestamps`, or `one-setting` naming the step.
  Anything else is `no`.
- `config` changes one response header in the peer's own config file, inside the container. A bind
  mount shows every file as world-writable, and Novis refuses such a config file, so Novis edits the
  `nvs.toml` its data folder holds. The change applied with no command is `built-in`, after the
  peer's documented reload command is `one-setting` naming it, and only after a restart is `no`.
  Bun, Go, Node.js, .NET and Python have no config file that sets a response header, so they run nothing and are
  `your-code`. ASP.NET Core's `appsettings.json` configures the server, but no setting in it adds a
  response header.
- No row replaces the runtime itself. Every server needs a restart or a reload for a new runtime
  version, so such a row says nothing a reader does not already know.

`--row <id>` runs one row, which is how a fix to one probe is checked.

## Agreement before timing

A peer that answers wrongly and fast is the easiest way for a benchmark to report a win. Each peer's
first rep of a case also takes its answers to a fixed set of requests, on the same stack, before the
timed load. After the last peer's first rep, the run checks that every peer agrees on status,
`Content-Type` and body, and only then starts rep 2. A disagreement stops the run, names the peer, and
writes no record, so a figure is never kept from a peer that answered wrongly. No stack is brought up
only to take answers: the answers cost a few requests on a stack the run needs anyway.

Two answers are compared after exactly two mappings and nothing else. The `Content-Type` loses its case
and spaces, since `charset=UTF-8` and `charset=utf-8` are one type. In a case that escapes text into
HTML, `&#039;`, `&#x27;` and `&apos;` become `&#39;`, and `&#34;` becomes `&quot;`, because each peer
escapes the way its own documentation says to and those spellings mean the same thing.

## Memory

Memory is the backend container's cgroup: `memory.current` before the first request and `memory.peak`
after the last. Every pass of a (case, peer) pair gets a fresh container, so the peak is that pass's alone and
never a figure left over from an earlier case. The proxy and the generator are not counted.

## One record, and no arithmetic with the proxied leg's

`benches/compare.json` is one JSON object, overwritten by each `--record` run: the page shows one run,
and the machine it was taken on is part of it. The feature rows are a second run into the same file:
`--scenarios --record` replaces the record's `scenarios` key with every row, under its own date and
commit, and a case run keeps the rows already there. So the two halves are re-run apart, and neither
needs the other's minutes. `--row` records nothing, because a recorded table is one run of every row. It shares the proxied leg's images but none of its
inputs — TLS, more peers, other cases and concurrencies — so no figure in it is ever divided by one in
`benches/serve-proxied.json`.

**A run is planned before it starts, and the record says where its time went.** `planOf` lists every
pass a run takes, and the run takes those passes and no others, so what `--plan` prints is the work
the run does: stacks brought up, images built, seconds of steady load and requests, per phase. The
count is the measure of a recording's cost, and a change to the harness that brings up more stacks says so before it runs. Wall clock is only a note: each case and
each row keeps the seconds of its steps under `seconds` (bringing the stack up, the answers or the
probe, the load, tearing it down), and a case keeps every peer's `passes` beside the median, so the
next recording can see where its time went and how far the passes spread. Seconds never gate anything,
since they move with the host.

**Each image is built once per run.** A run's first passes build every image a peer builds from this
checkout, and no stack after that comes up with `--build`. Every stack of the run then serves the same
image, and the record's commit names the tree it was built from. The baseline the cuts are measured
against is `FOUR_PEER_BASELINE`: 247 stacks and 203 builds for the four-peer recording, before the
answers moved onto rep 1 and the builds onto the start of the run.

**A row result is kept while nothing it depends on has changed.** A `--scenarios --record` run keeps a
peer's recorded result for a row when the row's key is unchanged, and runs every other pair. The key
is four parts: the id of the peer's image on this machine, a digest of the row's program folder, a
digest of `bench-compare.ts`, `bench-docker.ts` and every tuning file the record digests, and a digest
of the host with its Docker version. The tuning files are in the key because a changed nginx config or
pool can change a result as much as a changed program. An image that is not on the machine has no id,
and its rows run. Novis's image has the commit in it, so its rows run in every recording. Each result
keeps the date, commit and key it was measured under in its row's `measured`, so the page can say how
old the oldest result is. `--fresh` runs every row again. The cases are never kept: a case's figures
are relations between peers, measured in one interleaved run, and a figure from another run would
compare two hosts' moods. `--plan` counts the kept results without docker: it takes every image as
unchanged except Novis's, and the host as the record's. A row run writes the record after each row,
so a run stopped partway leaves its finished rows. The next run without `--fresh` keeps them, since
their keys are unchanged, and runs only the rest.

**`--peer ID,...` runs the named peers**, for checking a new peer's programs or a row cheaply. With one
peer, the answers are compared with nothing, so a new peer is smoked beside one that is already
checked, such as `--peer node,python`. `--record` refuses it, because a record is one run of every peer.

**Every figure is the median of interleaved runs.** A case runs `--reps` passes, five by default, and
the peers take turns: pass 1 of every peer, then pass 2, each on a fresh stack. The record keeps the
median pass's throughput and p99, the median of the passes' peaks, and the errors of all of them. The
runaway rows run three times and keep the median run's load; the deploy rows run three times and sum
their replies. The host runs other work, and the fastest of three let one lucky pass set a peer's bar,
while a slow minute fell on whichever peer's block it hit. `cores` and `waiting` take the median of
their passes too, one peer after another, because a row asks for its passes one budget at a time.

**The numbers are relations.** The absolute figures depend on the machine. The record names the machine
so the page can say which, and the page shows each peer against the best in each case.

**A record is taken from a clean tree.** Both halves run in a detached worktree at `HEAD`, made with
`git worktree add --detach .agent-tmp/worktrees/compare HEAD`, so another session's half-written edit
is never in an image. The record's two `commit` values then name a tree that exists, and the worktree's
`benches/compare.json` is copied over the tree's. Other work may go on while it records, because the
peers take turns within a run.

## The direct measure: every HTTP peer with no proxy

`--direct` answers what a request costs each server in CPU time, which the proxied figures hide behind
nginx and TLS. It runs every peer that listens on HTTP on the `hello`, `json-api` and `html-page` cases.

- **No nginx and no TLS.** Each backend runs alone in its own container, from the image and program its
  compose file names, with one worker. `oha` sends plain HTTP inside that container's network.
- **One cpu on its own core, and `oha` on other cores.** The backend gets `--cpus 1` on one core, and
  `oha` gets up to four cores the backend never uses. A backend that shares a core with the generator
  measures the generator too.
- **CPU time per request, from the cgroup.** Each pass reads the backend's `cpu.stat` before and after,
  and divides what it spent by the requests answered 2xx. The line gives the median requests per second
  over the passes, the microseconds per request split into user and system time, and the periods the
  quota throttled. The containers have no healthcheck, because a check runs in the backend's cgroup and
  its CPU time would be counted.
- **Warmed, then interleaved.** Each backend first gets 20,000 requests that are not counted. Then five
  passes of 10 s at 64 connections run with the peers taking turns: pass 1 of every peer, then pass 2.
- **PHP is absent.** PHP-FPM speaks only FastCGI and has no HTTP listener, so it cannot run without a
  proxy in front of it.
- **Nothing is recorded, and it is never a gate.** The figures depend on the machine and are read by a
  person. `--direct --smoke` sends a few hundred requests to each peer in each case once, and checks
  that every one is answered 2xx.

## The profile: Novis alone, on one pinned core

`--profile CASE` answers where Novis spends a request, which no record says. It runs `profile.sh` in one
privileged container of the Novis image, and measures no other peer.

- **No nginx and no TLS.** The proxy's cost is the same for every peer, so it says nothing about
  Novis. Without it, the server's own work is most of every sample.
- **One core serves, and other cores load.** `taskset` pins `nvs serve` to one core, which also makes
  it start one worker, and `oha` runs on cores the server never uses. A profile of a core that the
  generator also runs on measures the generator.
- **Privileged, because `perf` and `strace` attach to another process.** The container is the bench
  image, so the binary is the one the comparison measures, with `linux-perf` and `strace` added.
- **Counts beside the samples.** `strace -c` gives the system calls per request, and the `wait` case
  gives the server cgroup's peak minus its idle memory, per request in flight, over two loads. A
  second load whose figure is higher shows a pool that grows. These are the counts a change to the
  request path is judged by. The samples only show where to look.
- **Nothing is recorded.** A profile is read by a person, and the next change makes it stale.

`perf` needs a kernel that lets a privileged container record. Where it cannot, the command exits 1,
and `profile.sh` is the recipe: run its steps on a Linux host.

## Running it

```sh
bun nv bench-compare --smoke                          # every peer and case, a few hundred requests
bun nv bench-compare --scenarios --smoke              # every feature row against every peer
bun nv bench-compare --record benches/compare.json    # the fair arm, one cpu per backend
bun nv bench-compare --scenarios --record benches/compare.json   # the feature rows, into that record
bun nv bench-compare --arm deployed --record PATH     # four cpus per backend
bun nv bench-compare --down                           # tear every stack down
bun nv bench-compare --profile json-api               # perf and strace on Novis serving one case
bun nv bench-compare --direct                         # every HTTP peer with no proxy: CPU per request
bun nv bench-compare --direct --smoke                 # the same, a few hundred requests per peer
```

It needs `bun` and Docker, and nothing else. The first run builds `nvs` for Linux inside the image,
as the proxied leg does. The run takes minutes, so it is not part of `nv verify` or CI.
