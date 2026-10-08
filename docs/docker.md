# Novis in a container

Official images are published to the GitHub Container Registry on every release:

```sh
docker pull ghcr.io/novis-lang/novis:0.0.1
```

One image holds one binary. `nvs` is the compiler, the server, the test runner and the CLI at
once, so `serve`, `run` and `test` are all the same tag with a different command —
[§ *Serving an application*](#serving-an-application) and [§ *Using it as a CLI*](#using-it-as-a-cli)
are two ways to run it, not two things to install.

The images are built by [.github/workflows/release.yml](../.github/workflows/release.yml) from the
same binaries the release archives carry, so `ghcr.io/novis-lang/novis:X.Y.Z` and
`nvs-X.Y.Z-linux-x86_64.tar.gz` contain the identical bytes. [docs/release.md](release.md) is the
release procedure itself and what it needs configured once.

## Which tag

| Tag | Moves | Use it when |
|---|---|---|
| `0.4.1` | never | **Production.** The only tag that always means one build. |
| `0.4` | on each patch of that line | You want compatible fixes and nothing else — see the note below. |
| `latest` | on each release | Trying Novis out. Never in a deployment. |
| `sha-abc1234` | never | Pinning to a commit rather than to a version. |

**There is deliberately no bare `0` tag**, and there will not be one before 1.0.
`rule:packaging/below-1-0-the-breaking-slot-moves-left` moves the breaking slot
left below 1.0: `0.MINOR` is what carries a breaking change, so a `0` tag would walk straight
across one. `0.4` is the compatible line today, exactly as `1.4` will be after 1.0 — at which
point a `1` tag appears, because by then `MAJOR` is the slot that breaks.

`latest` and the `MAJOR.MINOR` line move only when a human publishes the release, which is
usually minutes after the version tag appears and occasionally much longer. `0.4.1` is pushed the
moment it is built.

### Variants

| | |
|---|---|
| `0.4.1` | **Default.** `gcr.io/distroless/cc-debian12` — about 38 MB on disk before the binary. No shell, no package manager, no `apt`. |
| `0.4.1-debian` | `debian:bookworm-slim` — about 114 MB before the binary, three times the default. The same binary, in an image with a shell you can `exec` into. |

Use the default. Reach for `-debian` when you need to get inside a running container, and prefer
reproducing the problem in it rather than shipping it: a debugging image that differs from the
production one in a second way is evidence about the wrong container.

Both are `linux/amd64` and `linux/arm64`; Docker picks for you.

## Serving an application

The image expects your application mounted at **`/app`**, with its entry point at
**`/app/main.nvs`**:

```sh
docker run --rm -p 8000:8000 -v "$PWD:/app:ro" ghcr.io/novis-lang/novis:0.4.1
```

That is the whole default command spelled out:

```
nvs serve /app/main.nvs --listen 0.0.0.0:8000
```

> **If you override the command, keep `--listen 0.0.0.0:8000`.**
> `rule:http-server/the-server-block-is-boot-class`'s default is
> `127.0.0.1:8000` — correct on a workstation, and inside a container it means the port is
> published, the process is running and every connection is refused. The failure reads like a
> Docker networking problem and is a loopback bind. The alternative is to write
> `listen = ["0.0.0.0:8000"]` under `[server]` in your `nvs.toml`, which the flag would then
> override anyway.

### Configuration

`nvs` reads `./nvs.toml` from its working directory, which is `/app`
(`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 2). So a mounted tree's own
configuration is found with nothing passed on the command line, and
[docs/reference/tools/20-config.md](reference/tools/20-config.md) is the reference for what goes
in it. With no `./nvs.toml`, step 3 reads `nvs.toml` in the data folder — `.nvsdata` beside the
binary, which is `/usr/local/bin/.nvsdata` in this image, or the folder `--data` names. The
`nonroot` account cannot create a folder in `/usr/local/bin`, so a container started without
`--data` prints the unusable-data-folder warning, runs without a compile cache and has no root for
`Core\IO::temporaryDir`; `--data` naming a writable volume, such as `/app/.nvsdata`, gives it all
three. Two keys matter more in a container than outside one:

```toml
[server]
listen      = ["0.0.0.0:8000"]  # or pass --listen; see the warning above
health_path = "/healthz"        # off by default, so no URL is silently reserved

[opcache]
file_cache_dir = "/var/cache/novis"  # optional; a writable volume here keeps compiled
                                     # artifacts across restarts (`rule:packaging/an-artifact-is-one-immutable-content-addressed-file`)
```

**Capabilities are denied by default and that does not change in a container.** A program that
reads files, opens sockets or connects to a database needs the grants in its `[[app]]` block —
`rule:security/capability-check-at-the-door` for `script.spawn`,
`rule:http-server/allow-url-pins-the-address` for `net`. Running as root in a container grants
nothing; the container boundary and the capability tree are unrelated mechanisms and both apply.

If you pass `--config`, note that naming any file **disables** both `nvs.toml` lookups entirely
(`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 1). That is deliberate — it is how you get a predictable tree — but it means a
mounted `nvs.toml` is silently unread once `--config` appears.

### Health checks

`health_path` is off until you set it. Once set it answers `200` while accepting and `503` while
draining, with an empty body, and performs **no dependency checks** — a probe that pings the
database turns a slow database into a simultaneous outage everywhere (`rule:http-server/the-server-block-is-boot-class`).

There is **no HTTP client inside either image**, so a `HEALTHCHECK` instruction and a Compose
`healthcheck:` have nothing to run. Neither image ships one, rather than shipping one that
silently fails. Use a probe that runs outside the container:

```yaml
# Kubernetes — the kubelet makes the request itself, so the image needs nothing.
readinessProbe:
  httpGet: { path: /healthz, port: 8000 }
livenessProbe:
  httpGet: { path: /healthz, port: 8000 }
```

Under Compose, put the check in whatever proxies to it, or add your own layer with `curl` in it.

### Scaling

**`nvs serve` is one process on one core**, and one listening socket (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`). It does not fork
workers, so a container with four CPUs runs one core's worth of Novis. Scale with replicas, and
give each container roughly one CPU:

```yaml
deploy:
  replicas: 4
  resources:
    limits: { cpus: '1.0', memory: 512M }
```

Put a proxy in front of them. `keepalive_timeout` must stay above the proxy's upstream keep-alive
or you get intermittent 502s — `rule:http-server/the-server-block-is-boot-class` owns that rule and the numbers.

### A Compose file that works

```yaml
services:
  app:
    image: ghcr.io/novis-lang/novis:0.4.1
    ports: ["8000:8000"]
    volumes:
      - ./app:/app:ro
      - novis-cache:/var/cache/novis
    read_only: true
    tmpfs: ["/tmp"]
    cap_drop: ["ALL"]
    security_opt: ["no-new-privileges:true"]
    # A little longer than `[server] drain_timeout`, which is 30s by default. See § Stopping a container.
    stop_grace_period: 35s

volumes:
  novis-cache:
```

`read_only` works because the process writes nothing outside `/tmp` unless you configure it to —
`Core\IO::temporaryDir()` is what needs the `tmpfs`, and `[cache] dir` is what needs the volume.
The container already runs as uid 65532; `cap_drop` and `no-new-privileges` cost nothing and are
worth setting anyway.

## Using it as a CLI

The entry point is `nvs` itself, so every subcommand is one word away:

```sh
# Run a script.
docker run --rm -v "$PWD:/app:ro" ghcr.io/novis-lang/novis:0.4.1 run hello.nvs

# Run a test tree, in CI.
docker run --rm -v "$PWD:/app:ro" ghcr.io/novis-lang/novis:0.4.1 test tests/

# Type-check without running.
docker run --rm -v "$PWD:/app:ro" ghcr.io/novis-lang/novis:0.4.1 check src/main.nvs

# Build and version information.
docker run --rm ghcr.io/novis-lang/novis:0.4.1 --version
docker run --rm ghcr.io/novis-lang/novis:0.4.1 info
```

Paths are resolved inside the container, and the working directory is `/app`, so `hello.nvs`
above is `/app/hello.nvs` — mount the directory you are standing in and the paths read the same
as they do on the host.

Two things to know when a command *writes*:

- **Mount read-write and match the uid.** The process is uid 65532, so a file it creates in a
  bind mount is owned by 65532 on the host. Add `--user "$(id -u):$(id -g)"` to get your own
  ownership back; nothing in the image needs the `nonroot` account specifically.
- **Arguments after the file go to the program**, not to `nvs`
  (`docker run … run script.nvs -- --flag`), which is `cargo run --`'s rule and is documented on
  `Core\Cli::arguments`.

## Stopping a container

`docker stop` sends `SIGTERM` to `nvs serve`. Both images set `STOPSIGNAL SIGTERM`. When the
server gets the signal, it does this:

- It accepts no new connections.
- If you set `[server] health_path`, that path returns `503`.
- Each request that is running finishes, and its response is sent.
- A connection with no request closes at once.
- When every connection is closed, the process exits.

`SIGINT` (Ctrl-C) and `SIGHUP` stop the server in the same way.

The server gives each open connection at most `[server] drain_timeout` to finish. The default is
`30s`. Docker waits 10 seconds by default, and then it sends `SIGKILL`. Kubernetes waits 30
seconds by default. Set the wait a little longer than `drain_timeout`:

- **Docker**: `docker stop -t 35 <container>`.
- **Compose**: `stop_grace_period: 35s`.
- **Kubernetes**: `terminationGracePeriodSeconds: 35`.

To stop faster, set a shorter `drain_timeout`, and a wait a little longer than it:

```toml
[server]
drain_timeout = "8s"
```

`nvs` is the first process in the container (pid 1). It handles `SIGTERM` itself, so you do not
need `docker run --init`.

## Verifying an image

Every image carries a signed statement that this repository's workflow, at a named commit,
produced that exact digest — keyless, so there is no key to leak:

```sh
gh attestation verify oci://ghcr.io/novis-lang/novis:0.4.1 --repo novis-lang/novis
```

That is the same verb the release archives use, because it is the same mechanism —
[docs/release.md](release.md) § *Verifying a published binary*. `latest` and `0.4` are additional
names for a digest that was already attested under its version tag, so verifying either verifies
the same statement.

Images are **not** stripped, for the reason the archives are not: `[profile.release]` keeps
`debug = "line-tables-only"` so a production backtrace names lines.

## Building an image locally

The Dockerfile compiles nothing — it packages a binary that already exists. Point it at one:

```sh
mkdir -p docker/bin/amd64 docker/bin/notices
cp target/x86_64-unknown-linux-gnu/release/nvs docker/bin/amd64/nvs
cp LICENSE THIRD-PARTY-LICENSES.txt README.md CHANGELOG.md docker/bin/notices/

docker build docker --target distroless --platform linux/amd64 -t novis:dev
```

`--target debian` builds the other variant. `docker/bin/` is ignored by git; the release workflow
fills it from the published archives, which is what makes the image's bytes and the archive's the
same bytes.

To build from source instead — a development image, not a release one —
[benches/proxied/Dockerfile](../benches/proxied/Dockerfile) already does that, with cargo cache
mounts and the toolchain pinned to `rust-toolchain.toml`.

## Why not Alpine or `scratch`

A ~15 MB static image is tempting and would break three things, each quietly:

- **`jiff` reads `/usr/share/zoneinfo` on Unix** and bundles the IANA database only on Windows. A
  `scratch` image has no tzdb, so every named zone — `America/New_York` — fails at runtime while
  everything else works.
- **DNS goes through `std::net::ToSocketAddrs`**, which is `getaddrinfo`. A musl build resolves
  with musl's own resolver instead, which is not the one this project tests against.
- **The musl build leg is `optional: true`** in the release matrix, because `ring` and `zstd-sys`
  compile C that needs `musl-gcc`. Shipping an image from a leg that is allowed to fail would
  make the image the least reliable artifact of the release.

`distroless/cc` gets most of the size benefit — no shell, no package manager, a third of the
`-debian` variant — with none of that. It is also worth knowing that **no CA bundle is needed**:
`nvs_host::tls` verifies
against Mozilla's set compiled into the binary (`webpki-roots`), so neither image installs
`ca-certificates` and outbound TLS works regardless.
