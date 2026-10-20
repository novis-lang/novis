# The proxied benchmark: nginx in front of both peers

`bun nv bench-proxied` runs this directory. It measures requests/sec for `nvs serve` against PHP 8.5
with opcache, in the shape both peers actually ship in, with the same generator, the same kernel and
the same CPU budget on either side.

**This file is the only home for why this leg is built the way it is.** `bun nv bench-load` is the
other server leg. It has no peer, and `tools/nv/cmd/bench-load.ts` owns its decisions. The two are
not one series and must never be read as one; § *One artifact, and no arithmetic across legs* below
is why.

## Why nginx fronts both, and why that is not a handicap

`rule:http-server/two-deployments-and-nothing-a-proxy-owns` gives `nvs serve` two
deployments and no third, and the production one is **a proxied origin that replaces FastCGI**: no
TLS listener, no h2c, no compression, no edge limiting, because a proxy in front does all of it
earlier and better. PHP-FPM is the same shape for the same reason — FPM speaks FastCGI and nothing
else, so *there is no deployment of it without a proxy*.

So nginx in front of both is not a fairness adjustment applied to a benchmark. It is the only
topology either peer has. A measurement with no proxy on either side sets our HTTP parse against
PHP's FastCGI frame, which is the artificial shape; arms 3 and 4 below take it only to price the proxy.

## The CPU budget, and the two arms

**Both peers use every CPU they are given.** `nvs serve` starts one worker per CPU it can see
(`[server] workers`, whose default is that count), and PHP-FPM starts its whole `pm.max_children`
pool. Inside a container both see every CPU of the host, so neither worker count is the limit. Docker
is what makes the comparison honest, and it is the reason this leg is containers rather than two
processes on the host: **a CPU budget is a compose key.** So there are two arms and each says what
it is:

- **`--arm fair` (the default)** — one CPU to each backend container. This is the
  runtime-against-runtime number on a single core.
- **`--arm deployed`** — `n` CPUs to each container, four unless `--backend-cpus` says otherwise.
  It is what an operator sees on a multi-core box, and the fair arm beside it shows how each peer
  scales from one CPU to `n`.

**Fairness is the CPU budget, and deliberately not the worker count.** The tempting version of the
fair arm — one core *and one PHP worker* against one Novis core — does not merely mislead, it does
not run: a FastCGI worker holding a keep-alive connection is *bound* to that connection and accepts
no other, so a single worker facing eight concurrent connections serves one of them and leaves the
rest accepted by the kernel and answered by nobody. It presents as a socket timeout while the
container's own log shows healthy `200`s going past.

So the pool is sized from `--concurrency` (`max(2 × concurrency, 96)`, rendered into
`generated/pool.conf` and recorded as `php_children`), which is also how a real FPM pool is sized,
and the equal thing between the two peers is the `cpus:` allowance the compose files set. Idle
workers cost memory and no CPU, so a generous pool is also the setting that favours PHP. The floor of
96 clears the 64 idle upstream connections nginx can hold (`worker_processes 4` × `keepalive 16`) —
a pool smaller than that starves itself the same way.

Both arms give the two stacks byte-identical budgets for nginx and for the generator, so the only
difference between them is the backend.

## Why two compose files rather than one with profiles

Because they must never be up at the same time. One file with two profiles is one `docker compose
up` away from both stacks competing for the same cores, and a run taken that way is not merely noisy
— it is wrong in a direction nothing in the output would reveal. Two files make "both at once" an
explicit act rather than a default. `bun nv bench-proxied` brings one up, measures it, tears it
down, and only then brings up the other.

## The arms, and the generator each uses

| # | Arm | Generator | What it is for |
|---|---|---|---|
| 1 | `oha` → nginx → php-fpm | `oha` | PHP as it ships |
| 2 | `oha` → nginx → `nvs serve` | `oha` | Novis as it ships |
| 3 | `oha` → `nvs serve` | `oha` | what nginx costs us |
| 4 | fcgi → php-fpm | `bun nv bench`'s FastCGI client | what nginx and the bridge cost PHP |

**`oha` and not our own generator.** The closed-loop generator in `tools/nv/cmd/bench.ts` exists for
a host where neither `wrk` nor `oha` is installed. In a container both are one pinned image away, so
it has no reason to run here — and `oha` reports the tail latencies that generator deliberately does
not.

**Arm 4 is the one asymmetry, and it is recorded rather than hidden.** FPM has no HTTP origin, so
"PHP without a proxy" cannot be driven by `oha` at all; it is driven over FastCGI by the client in
`tools/nv/cmd/bench.ts`, imported rather than written twice. That makes arm 3 → 1 and arm 4 → 2 a *within
stack* proxy cost each, and it makes arm 3 against arm 4 a comparison across two generators. Every
record says so in its `caveats`.

## One artifact, and no arithmetic across legs

This leg writes `benches/serve-proxied.json`, and nothing else writes it. `bun nv bench-load`
records into a file of its own, because every input differs: operating system, generator, topology,
CPU budget and container runtime. A figure taken at one concurrency and one taken at another can
never be read as one series, and separate files are how that is enforced rather than remembered.

**The ratio includes nginx.** nginx adds a cost both sides pay, so the proxied ratio is closer to 1
than an origin-against-origin one. That is the benchmark measuring the deployment, not the server
getting slower, and a row from this file is compared only with an earlier row from this file.

## Tuning is a variable we own, so it is written into every row

`pm.max_children`, the opcache and JIT settings, nginx's `worker_processes` and both upstream
keepalive pools are all ours to set, and a benchmark of oneself whose peer is badly tuned is not a
benchmark. Two rules keep it honest:

- **Every setting that could flatter either side is in `php/` or `nginx/` in this directory and in
  git**, and every record carries a `config_digests` map — a short sha256 per file — beside the
  commit, plus the handful of opcache keys that move a number spelled out in `php_ini`. Digests
  rather than the files' own text, because the artifact keeps a hundred runs and those four files
  are mostly comments; the digest and the commit together recover the exact bytes from git on the
  day a row is actually questioned, and a run taken after an untracked edit is visibly a different
  run rather than a mysterious one.
- **The PHP stack is tuned first and to its own advantage** — `static` process management so there
  is no ramp, `opcache.validate_timestamps=0` so it never stats, JIT on. Where a setting has a
  defensible range, this directory takes the end that favours PHP.

Image pinning, the absence of data volumes and the loopback-only publishes follow
`tests/db/compose.yaml`'s header, which is the home of those three standing decisions for every
compose file in this repository. `oha` is pinned by digest because upstream publishes no version
tag.

## Running it

```sh
bun nv bench-proxied                                  # the fair arm, both stacks
bun nv bench-proxied --arm deployed --backend-cpus 8  # both peers on eight cpus
bun nv bench-proxied --record benches/serve-proxied.json
bun nv bench-proxied --nvs-bin /var/tmp/nvs-target-wsl/release/nvs   # skip the image build
bun nv bench-proxied --down                           # tear both stacks down and stop
```

The first run builds `nvs` for Linux inside the image and costs a cold release build; every run after
it reuses BuildKit's cargo and target caches. `--nvs-bin` skips the build entirely and takes a binary
the WSL leg already produced — `docs/agent/commands.md` § *Fuzzing and callgrind on Windows* is where
that target directory is named. The binary must be a **release** build and must be Linux: the driver
refuses anything else rather than reporting a debug build as a result.
