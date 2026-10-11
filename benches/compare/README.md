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
cores and not the container's quota; `nvs serve` with its defaults.

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

- `cores` and `waiting` send no requests of their own. Each times a case the way a full run does, on
  a fresh stack per pass, with the run's `--requests` and `--reps`.
- `cores` times html-page at one cpu and at four. Throughput that grows by 2x or more is `built-in`,
  or `your-code` where the peer's program carries a launcher to use the cores, as Bun's `serve.ts`
  does. Less than 2x is `no`. nginx and `oha` each have four cpus, and hello reaches their ceiling
  at four backend cpus with every peer, so the row times the case with the most work per request.
  A peer whose four-cpu figure still nears that ceiling measures the front, and the evidence keeps
  both figures so a reader can see it.
- `crash` sends one request that throws or panics, then one that does not. The second must answer
  `200` from a backend that is still running. No stack sets a restart policy, so a crashed backend
  stays down and shows.
- `loop` and `memory` each put 15 seconds of steady load on `/` and, 3 seconds in, send one runaway
  request into it: `loop` sends `/loop`, which never ends, and `memory` sends `/alloc`, which
  allocates 1 MiB at a time without end. Each backend container has a 512 MiB cap in these rows, so
  an allocation without end stops at the container and not at the host. The runaway request counts
  as stopped when the backend answers it within 55 seconds, under nginx's 60-second read timeout,
  with any status but nginx's own `502` or `504`. No failed request under load, the runaway stopped
  and the backend still running is `built-in`. A request `oha` still has open when the load ends is
  not a failure. Novis sets no limit by default, so it writes `memory = "16M"` and `cpu_time = "1s"`
  in its `[limits]` table first and is `one-setting`. `nvs serve` admits a request only while the
  requests already running leave room in the container's memory for it to reach `memory`. PHP's own `max_execution_time` and `memory_limit` are on by
  default, so PHP is `on-by-default` naming the one its row needs. The other answers are `no`, and
  the load's p99 stays in the evidence.
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
  the copy, and polls for 20 seconds. Each peer runs at its own defaults. The speed tests set
  `opcache.validate_timestamps=0` for PHP, so before the first version the row writes PHP's default
  `1` in a later ini and runs `php-fpm reload`; an edit that then shows is `on-by-default` naming it.
  Bun runs without `--hot`, its default. An edit that never shows is `one-setting` where the peer's
  documentation names a setting it starts without, and the measured `null` stays in the evidence. Go
  compiles its program into the image, so it runs nothing and is `no`.
- `deploy` serves a copy whose program reads 200 part files, each printing its version, so a reply
  shows every part's version. The driver writes the parts, and Novis's entry too, because `require`
  takes only a path written as literal text. PHP runs at its default `opcache.validate_timestamps=1`,
  restored as the `live-edit` row restores it. Two seconds into 20 seconds of steady load, it
  replaces the parts one at a time with a 10 ms pause, then runs the step the peer needs at its
  defaults: none for Novis and PHP, which check their files, and a restart for Bun and Go. Go
  compiles its parts into the image, so it only restarts, and its reply is not checked for a
  version. The row's own requests run the whole time and count replies that mix the two versions.
  No failed request, no mixed reply and the new version within 20 seconds of the deploy step is
  `built-in` with no step, `on-by-default` naming PHP's `opcache.validate_timestamps`, or
  `one-setting` naming the step. Anything else is `no`.
- `config` changes one response header in the peer's own config file, inside the container. A bind
  mount shows every file as world-writable, and Novis refuses such a config file, so Novis edits the
  `nvs.toml` its data folder holds. The change applied with no command is `built-in`, after the
  peer's documented reload command is `one-setting` naming it, and only after a restart is `no`.
  Bun and Go have no config file that sets a response header, so they run nothing and are
  `your-code`.
- `upgrade` replaces the runtime binary two seconds into 20 seconds of steady load. The new file is
  written beside the old one and renamed over it, since a running binary cannot be written in place.
  Then the peer's documented step makes the new binary run: `php-fpm reload`, which re-executes
  PHP-FPM's own binary, and a restart for Novis, Bun and Go. Go's runtime is compiled into the
  program, so its binary is the program. Every peer needs a step, so no failed request is
  `one-setting` naming it, and anything else is `no`.

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
after the last. Every (case, peer) pair gets a fresh container, so the peak is that arm's alone and
never a figure left over from an earlier case. The proxy and the generator are not counted.

## One record, and no arithmetic with the proxied leg's

`benches/compare.json` is one JSON object, overwritten by each `--record` run: the page shows one run,
and the machine it was taken on is part of it. The feature rows are a second run into the same file:
`--scenarios --record` replaces the record's `scenarios` key with every row, under its own date and
commit, and a case run keeps the rows already there. So the two halves are re-run apart, and neither
needs the other's minutes. `--row` records nothing, because a recorded table is one run of every row. It shares the proxied leg's images but none of its
inputs — TLS, four peers, other cases and concurrencies — so no figure in it is ever divided by one in
`benches/serve-proxied.json`.

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
