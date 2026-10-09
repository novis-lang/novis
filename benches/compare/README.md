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
and the machine it was taken on is part of it. It shares the proxied leg's images but none of its
inputs — TLS, four peers, other cases and concurrencies — so no figure in it is ever divided by one in
`benches/serve-proxied.json`.

**The numbers are relations.** The absolute figures depend on the machine. The record names the machine
so the page can say which, and the page shows each peer against the best in each case.

## Running it

```sh
bun nv bench-compare --smoke                          # every peer and case, a few hundred requests
bun nv bench-compare --record benches/compare.json    # the fair arm, one cpu per backend
bun nv bench-compare --arm deployed --record PATH     # four cpus per backend
bun nv bench-compare --down                           # tear every stack down
```

It needs `bun` and Docker, and nothing else. The first run builds `nvs` for Linux inside the image,
as the proxied leg does. The run takes minutes, so it is not part of `nv verify` or CI.
