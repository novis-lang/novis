# The comparison: Novis, PHP, Bun and Go behind one TLS proxy

`bun nv bench-compare` runs this directory. It measures four servers end to end — `nvs serve`, PHP-FPM,
`Bun.serve` and Go's `net/http` — each behind the same nginx terminating TLS 1.3, on the same CPU
budget, with the same generator. Its record is `benches/compare.json`, and the website's comparison
page is rendered from it.

**This file is the only home for why the comparison is built the way it is.** What it shares with the
proxied leg — nginx in front, one stack up at a time, the CPU budget and its two arms, `oha` inside the
stack's network, tuning in git with a digest in every row — is decided in
[benches/proxied/README.md](../proxied/README.md), and nothing here restates it. How a run is carried out
is `tools/nv/cmd/bench-compare.ts`'s header.

## Why these four peers

Novis against the three runtimes a team choosing a web server is likely to weigh it against. PHP,
because FPM is the deployment Novis replaces. Bun, as the JavaScript peer: Node.js is the same language
and would add a column that says little new. Go, as the compiled peer with a standard-library server.
Python is absent because `rule:tooling/python-claims` forbids the claim such a column would make.

Each peer uses its standard library only, and is tuned to its own advantage with every setting in git:
FPM with a static pool, opcache and the JIT; Bun with one process per CPU of the budget sharing the
port with `reusePort`; Go with `GOMAXPROCS` set to the budget, because the Go runtime reads the host's
cores and not the container's quota; `nvs serve` with its defaults. The one exception is PHP's file
check, which stays on at PHP's defaults (`php/file-check.ini`). The page groups PHP with Novis as a
server whose copied files are live, and with the check off a copied file is not live until a reload.

## Why TLS, and why at nginx

Every peer is deployed behind a proxy that terminates TLS, and that is the path a browser's request
takes. Novis has no TLS listener at all (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`),
so a comparison without the proxy would measure a deployment Novis does not have. The proxy's cost is
in every number, and it is the same cost for every peer: one nginx config for the three HTTP backends
and one for FastCGI, kept line-for-line parallel so a diff shows the backend and nothing else.

TLS 1.3 only, HTTP/1.1, keep-alive on both sides of the proxy. The certificate is self-signed, made
once by `alpine/openssl` into the git-ignored `generated/tls/`, and `oha` runs with `--insecure`.

## One compose file per peer, and a common one

Each peer has its own `compose.<peer>.yaml`, for the proxied leg's reason: two stacks up at once is a
deliberate act. nginx and `oha` are declared once in `compose.common.yaml` and taken with `extends`, so
the proxy and the generator cannot drift between peers. Each peer publishes nginx on its own loopback
port, so a stack left running names itself.

The peers are data. `PEERS` in `tools/nv/cmd/bench-compare.ts` names each one's compose file, nginx
config, port, image and the settings written into its rows. A fifth peer is a new entry and a compose
file, never a new code path.

## One program per case per peer

A case is a folder under `cases/` with one program per peer: `main.nvs`, `index.php`, `server.ts` and
`main.go`. The stack mounts or builds the case it is brought up for, so no peer carries a router the
others do not. Go compiles every case into its image once (`go/Dockerfile`), and Bun runs the case's
`default { fetch }` under `bun/serve.ts`.

`json-api` is built so its four answers are byte-equal. Money is integer cents, and the 20 percent tax
is rounded half up by integer division, so no peer's float formatting shows. Keys are written in one
fixed order. The items are sorted by a stable sort, largest total first, and `order.json` has two items
with one total, so a peer whose sort is not stable disagrees. Every failed check answers 422 with one
fixed body. Each peer checks types in its own language's usual way: Novis and Go decode into typed
classes, PHP and Bun test the decoded value. The rules are the same set, never the same code.

`html-page` builds its 100 products again on every request and escapes each name and the search term
the way each peer's own documentation says to: an html template in Novis, `htmlspecialchars` in PHP,
`Bun.escapeHTML` in Bun and `html/template` in Go. The timed term holds `<`, `>`, `&`, `"` and `'` and
no `+`, because `html/template` alone also escapes `+` and that difference is not one the comparison
maps away. Prices are integer cents formatted with integer division, for `json-api`'s reason.

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

A row is a folder under `scenarios/` with the same four programs a case has, and a probe in
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
PHP's `<?=`, a Bun template literal, Go's `html/template` and a Novis html template. On `/direct` it
uses the plainest write to the response: PHP's `echo`, a Bun string joined with `+`, Go's
`fmt.Fprintf` and a Novis `echo` of strings. The row is `built-in` only when both arrive escaped,
because a safe way that sits beside an unsafe one is a choice the developer has to get right.
Otherwise the developer chooses the safe way or calls an escaper, so the result is `your-code` and
not `no`.

- `cores` and `waiting` send no requests of their own. Each times a request the way a full run does,
  on a fresh stack per pass, with the run's `--requests` and `--reps`.
- `cores` times one part of the `parallel` row's loop, about a millisecond of one core, at one cpu
  and at four. Throughput that grows by 2x or more is `built-in`, or `your-code` where the peer's
  program carries a launcher to use the cores, as Bun's `serve.ts` does. Less than 2x is `no`. A case
  in `cases/` cannot measure this: nginx and `oha` each have four cpus, and a peer that answers html-page
  at 20k requests a second on one cpu meets their ceiling at four, so its scaling would be the front's.
  Four cpus of a millisecond each answer a few thousand requests a second, far under that ceiling.
- Every backend has the restart policy `unless-stopped`, because every peer is deployed under a
  supervisor that starts a server again when it exits. A row reads the container's restart count, so a
  crash still shows, and the load after it measures the restart rather than a server left down.
- `crash` sends one request that throws or panics, then one that does not. The second must answer
  `200` from a backend that is running and was never restarted.
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
  measures the other requests while the runaway one still runs. Bun and Go have no limit for one
  request, so their answers are `no`, and the load's p99 stays in the evidence. Bun's `loop` is
  `your-code` (`LOOP_IN_CODE`): `Worker.terminate()` stops a worker that never yields, so a program
  that runs each request in a worker under a time limit stops it. Bun ignores a worker's
  `resourceLimits`, measured with `oven/bun:1.4.3-slim` and stated in its Node compatibility page, so
  `memory` has no such way. No framework on either side stops a goroutine or a blocked event loop: Gin,
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
  are stopped with `500`. The nightly deep lane runs the row's `--smoke` form, which fails only when
  a stack cannot serve the row.
- No row tests whether a request sees a value another request set. A process that keeps state
  between requests is a design choice in Bun and Go, not a failure, and a yes or no cannot say which
  of the two a reader wants.
- `taint` and `secret` run `nvs check` over a program that puts a request value into an SQL string,
  and over one that prints a secret, in a one-off app container with no stack up. The check must
  fail with the row's own error code, so a program that fails for another reason is not counted.
  PHP, Bun and Go have no such check, so they run nothing. Their result is `no`, and the evidence
  links the page in their own documentation that warns about the mistake.
- `csrf` sends two `POST`s to a route that changes state: one with no `Origin`, which must change
  it, and one with `Origin: https://other.example`. The second is rejected when its reply does not
  say the state changed, whatever the status. Accepted is `your-code`, because a check of the
  `Origin` or a token is code or a library the developer adds.
- `session` starts a session, sends `/slow`, which writes the session and then works for two
  seconds, and 200 ms later `/fast` with the same cookie, which only reads it. `/fast` answered in
  under a second is `built-in`. Longer is PHP's session lock holding it until `/slow` ends, and
  `your-code` names the two calls that release it early. Bun and Go have no session without a
  library, so they run nothing and are `your-code`. The row does not test lost writes: with no lock,
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
  parts, Go starts goroutines, Bun starts `Worker` threads and Novis spawns child scripts
  `on: "worker"`. PHP-FPM has no way to do it, so its parts run one after another. A speedup of 2x
  or more over four parts in a row is `built-in`. Novis denies `spawn script` by default, so it
  writes `spawn = true` in its `[capabilities.script]` table first and is `one-setting`. Less than
  2x is `no`.
- `live-edit` serves a copy of its folder under `generated/`, changes `version 1` to `version 2` in
  the copy, and polls for 20 seconds. Each peer runs at its own defaults. PHP runs every case and
  row with `opcache.validate_timestamps=1` and `revalidate_freq=2`, its defaults, from
  `php/file-check.ini`, so an edit that shows is `on-by-default` naming the setting. Bun runs
  without `--hot`, its default. An edit that never shows is `one-setting` where the peer's
  documentation names a setting it starts without, and the measured `null` stays in the evidence. Go
  compiles its program into the image, so it runs nothing and is `no`.
- `deploy`, `deploy-big` and `deploy-switch` serve a program that reads part files, each
  printing its version, so a reply shows every part's version. `deploy/deploy.sh` writes the parts, and
  Novis's entry too, because `require` takes only a path written as literal text. It runs inside the
  container and writes to a Docker volume, because Docker Desktop's file share reports thousands of
  changed host files late, and a Linux server has no such share. The PHP, Bun and Go
  programs read the part count from `DEPLOY_PARTS`. PHP runs at its default
  `opcache.validate_timestamps=1`, as in every case and row. The load is the driver's own: 8 loops of
  `GET /`, each reply checked, because `oha` never reads a body and a sampler beside it checks a few
  replies out of many. A reply is failed (an error, or any status but `200`), mixed (more than one
  version in it) or complete. Two seconds in, the driver writes version 2 one file at a time with a
  10 ms pause, then runs the step the peer needs at its defaults: none for Novis and PHP, which check
  their files, and a restart for Bun and Go. The load stops a second after the first reply of the
  new version, or 20 seconds after the step without one. Go compiles its parts into the image, so it
  only restarts, and its new version is its first reply to a request sent after the restart.
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
  Bun and Go have no config file that sets a response header, so they run nothing and are
  `your-code`.
- No row replaces the runtime itself. Every server needs a restart or a reload for a new runtime
  version, so such a row says nothing a reader does not already know.

`--row <id>` runs one row, which is how a fix to one probe is checked.

## Agreement before timing

A peer that answers wrongly and fast is the easiest way for a benchmark to report a win. A full run
brings every (case, peer) pair up once only to take its answers to a fixed set of requests, and times
nothing until the four agree on status, `Content-Type` and body. A disagreement stops the run and names
the peer.

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
inputs — TLS, four peers, other cases and concurrencies — so no figure in it is ever divided by one in
`benches/serve-proxied.json`.

**Every figure is the median of interleaved runs.** A case runs `--reps` passes, five by default, and
the peers take turns: pass 1 of every peer, then pass 2, each on a fresh stack. The record keeps the
median pass's throughput and p99, the median of the passes' peaks, and the errors of all of them. The
runaway rows run three times and keep the median run's load; the deploy rows run three times and sum
their replies. The host runs other work, and the fastest of three let one lucky pass set a peer's bar,
while a slow minute fell on whichever peer's block it hit. `cores` and `waiting` take the median of
their passes too, one peer after another, because a row asks for its passes one budget at a time.

**The numbers are relations.** The absolute figures depend on the machine. The record names the machine
so the page can say which, and the page shows each peer against the best in each case.

## Running it

```sh
bun nv bench-compare --smoke                          # every peer and case, a few hundred requests
bun nv bench-compare --scenarios --smoke              # every feature row against every peer
bun nv bench-compare --record benches/compare.json    # the fair arm, one cpu per backend
bun nv bench-compare --scenarios --record benches/compare.json   # the feature rows, into that record
bun nv bench-compare --arm deployed --record PATH     # four cpus per backend
bun nv bench-compare --down                           # tear every stack down
```

It needs `bun` and Docker, and nothing else. The first run builds `nvs` for Linux inside the image,
as the proxied leg does. The run takes minutes, so it is not part of `nv verify` or CI.
