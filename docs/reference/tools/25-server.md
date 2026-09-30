---
id: server
title: The HTTP server
summary: `nvs serve`, the `[server]` block and its mounts, how a request finds the file that answers it, what a program reads about the door it came through, what reaches a running server, the drain, `nvs ctl` and `nvs service`
keywords: nvs serve, server, HTTP, restart, restart required, deploy, hot reload, zero downtime, symlink, settle, revalidate_freq, listen, port, --listen, --port, [server], [[server.mount]], mount, prefix, host, scan, entry, origin, root, dispatch, static, static files, trusted_proxies, X-Forwarded-For, X-Forwarded-Proto, client IP, health_path, health check, max_in_flight, workers, timeout, drain, drain_timeout, graceful shutdown, reload, nvs ctl, control socket, nvs service, service, systemd, Windows service, Core\Request::mount, Core\Request\Mount, Core\Router::url, multi-tenant, subdirectory, virtual host, front controller, try_files, php -S, php-fpm, nginx, Apache, .htaccess, RewriteBase, SCRIPT_NAME, PATH_INFO, DocumentRoot
---

# nvs serve

    nvs serve [<file>] [--listen <host:port> | --port <n>] [--config <path>]...

`nvs serve` runs an application until the process is stopped. It answers HTTP requests on every
core, runs the `[[schedule]]` entries on the core that ticks, and drains the `[queue]` workers
beside them; a configuration that writes neither starts neither. Every file it serves is compiled
before the socket is bound, so a program with a diagnostic never answers a request, and every
request runs in an isolate that shares nothing with any other request.

The development server and the production origin are the same command. In production it sits
behind a proxy on loopback or a Unix socket; nothing a proxy does earlier — TLS, compression,
rate limiting — is in it.

What is served depends on the file named and on whether the configuration writes a mount table:

| Start | What is served |
|---|---|
| `nvs serve app.nvs`, no `[[server.mount]]` written | `app.nvs` at `/`, its own directory as the mount root |
| `nvs serve`, mounts written | every mount in the table, under the global configuration |
| `nvs serve app.nvs`, mounts written | the table, and `app.nvs` must be one of its entries; the `[[app]]` blocks matching it apply to every mount |
| `nvs serve`, no mounts written | refused: nothing was named |

`--listen` replaces `[server] listen` for this run, and `--port` keeps the host the file chose and
replaces the port; the two are not combined. `--config` reads a configuration file instead of
`./nvs.toml` (the configuration chapter). With nothing written anywhere the server listens on
`127.0.0.1:8000`.

# The `[server]` block

```toml
[server]
root               = "/srv/app"           # every mount path resolves inside this; required with a mount table
listen             = ["127.0.0.1:8000"]   # "host:port" entries, or an absolute path for a Unix socket
socket_mode        = "0660"               # Unix-socket entries only
dispatch           = "entry"              # "entry" or "path"; unset is "entry", or "path" in development
static             = false                # serve files under the mount root; unset is false, or true in development
trusted_proxies    = []                   # empty: forwarding headers are never read
health_path        = ""                   # "" is off
max_in_flight      = 10000                # requests in flight before a 503
workers            = 4                    # accept cores; unset is the machine's parallelism
header_timeout     = "10s"                # idle waits, each finite with nothing written
body_idle_timeout  = "30s"
write_idle_timeout = "30s"
keepalive_timeout  = "75s"
drain_timeout      = "30s"                # how long working connections are served after a stop begins
```

A running server applies a change to any key here by itself, except `listen`, `socket_mode` and
`workers`. A change to one of those three takes effect at the next start (§ *What reaches a running
server*). `listen` is one flat list. An entry that begins with a path separator is a Unix socket, which is
Unix-only and is the transport to prefer behind a proxy; `socket_mode` is who may connect to it.
On Windows the server listens on TCP.

`dispatch` and `static` are the two keys the mode selects a default for. Left unwritten, they take
the value of the `[mode] default` the file names, each time the configuration is loaded or
reloaded: a development server dispatches by path and serves static files, and a production server
does neither. A mode switch made while the server runs changes neither key, and no request may
change either. `workers` is how many cores accept; every core holds its own handle on every
listener. `max_in_flight` answers a fixed `503` with `Retry-After` before any program runs. The
four waits are idle waits, never a total, and none can be turned off. `[server.connection]`
bounds a connection that was upgraded out of a request — a WebSocket, or an event stream — with
`max_open`, `max_frame`, `max_message`, `idle_timeout`, `max_lifetime` and `send_timeout`, all
finite with nothing written.

# Mounts: which file answers a request

A mount maps a URL onto one entry file under `[server] root`. It matches on `prefix`, on `host`,
or on both, and names either `entry` (one file) or `scan` (a glob):

```toml
[server]
root = "/srv/www"

[[server.mount]]
scan   = "*/public/index.nvs"             # one mount per match; * captures one path segment
prefix = "/{1:lower}"                     # {1} is the first capture; :lower is the only transform
origin = "https://{1:lower}.example.com"  # what Core\Router::urlAbsolute prepends here

[[server.mount]]
prefix = "/admin"
entry  = "Backoffice/public/index.nvs"    # an explicit mount overrides a scanned one at the same key

[[server.mount]]
host   = "shop.example.com"               # compared without regard to case
entry  = "Shop/public/index.nvs"
```

- A `scan` expands against the disk into ordinary mounts when the server starts. It expands again
  when a scanned directory changes, and when a reload changes `root` or a mount. `*` matches exactly one segment of letters, digits, `.`, `_` and `-`, not
  beginning with a dot. `{n}` is the nth capture as the directory spells it, `{n:lower}` the same
  in lower case, so a directory named `Blog` is served at `/blog` while `Core\Request::mount()`
  still reports `Blog`. Any other brace is an error at start and under `nvs config check`.
- `root` is required whenever a mount is written, and every resolved path must lie inside it.
  A relative `root` resolves against the file it is written in.
- Two explicit mounts at one key are an error. A scan that matched no file refuses the start.
- A mount routes and nothing else: it has no `mode`, no limits and no capabilities. Those are the
  `[[app]]` block's, keyed on the entry file's path (the configuration chapter), and the two
  usually name the same directory.

<!-- src: `rule:http-server/a-mount-table-expands-at-boot`, `rule:http-server/a-mount-carries-no-policy` -->

## How a request resolves

```
1. longest match:  host mounts by prefix  ->  host-less mounts by prefix  ->  404
2. strip the prefix
3. static = true   and the rest names an existing non-.nvs file under the mount root  -> serve it
4. dispatch = "path" and the rest names an existing .nvs file under the mount root    -> run it
5. otherwise                                                                          -> run the mount's entry
```

In production — `dispatch = "entry"`, `static = false` — steps 3 and 4 do not run: match, strip,
entry. With `dispatch = "path"` and `static = true`, which is what development gives when neither
key is written, the sequence is `try_files $uri /index.nvs`, the shape a PHP application already
deploys under. A prefix is matched exactly, and in steps 3 and 4 the URL has to write the name of
the file exactly as the disk has it. A name in another case, a name with a dot or a space added at
its end and a Windows short name such as `REPORT~1.NVS` are each a file that is not there, on every
platform. A trailing slash is never added or removed: `/users` and `/users/` are two URLs. `HEAD`
runs as `GET` with the body discarded.

`health_path`, when set, is matched ahead of every mount: `200` with an empty body while the
process accepts, `503` while it drains, no dependency checked and nothing logged.

<!-- src: `rule:http-server/a-request-resolves-in-five-steps`, `rule:http-server/health-path-is-off-and-checks-nothing` -->

## Static files

With `static = true`, step 3 serves the exact file and nothing else: no directory listing,
`index.html` as the only default document, the type from a fixed extension table and
`application/octet-stream` for an unknown one. Freshness is `Cache-Control: no-cache` with one
strong `ETag`; one `Range` is honoured and anything else is a `416`. A `.nvs` file is never served
as source, under any setting, from any mount. This is a development convenience and a fallback,
not a CDN: there is no `max-age` and no precompressed lookup.

<!-- src: `rule:http-server/static-serving-is-one-policy` -->

# What a program reads about its door

The matched prefix is stripped before the program runs, so `Core\Request::path()` is the
remainder and one compiled program serves at `/`, at `/shop` or at `{1}` with no change.
`Core\Request::mount()` returns the rest: `prefix()` is what was stripped, `captures()` the glob
captures of the mount that matched, in order, `tainted` because which mount answers is the
client's choice. It is never `null` — a program served without a mount table reads `""` and an
empty array — and outside a request it throws.

```nvs skip
<?nvs
// Mounted by `scan = "*/public/index.nvs"` at `prefix = "/{1:lower}"`, so
// `/acme/orders` arrives here with `path()` equal to `/orders`.
Core\Request\Mount $mount = Core\Request::mount();
tainted string $tenant = $mount->captures()[0];
echo "serving ", $tenant, " under ", $mount->prefix(), "\n";
```

`Core\Router::url` puts the same prefix in front of every link it builds, so a link is never
assembled from a route's declared path by hand, and `Core\Router::urlAbsolute` prepends the
mount's `origin`, falling back to the `[[app]]` block's. A program run from the command line is
mounted nowhere: the prefix is empty and nothing else changes.

<!-- src: `rule:routing/a-request-reads-its-mount`, `rule:routing/link-carries-the-mount-prefix` -->

# Behind a proxy: `trusted_proxies`

```toml
[server]
listen          = ["127.0.0.1:8000"]
trusted_proxies = ["10.0.0.5", "10.1.0.0/16"]   # one address, and one block of addresses
```

`trusted_proxies` lists the proxies whose forwarding headers the server reads. An entry is one
address or one block of addresses, IPv4 or IPv6. The server prints a note at start for an entry
that is neither, and does not use that entry. A change to the list applies to the next request,
without a restart.

**The list is empty.** This is the default, and the server reads no forwarding header.
`Core\Request::clientIp()` returns the address that connected, and `Core\Request::scheme()` returns
`http`. A program can still read `X-Forwarded-For` with `Core\Request::header`, as a `tainted`
string.

**The list has the address that connected.** `clientIp()` returns the last address in
`X-Forwarded-For` that is not in the list. A client can send its own `X-Forwarded-For`, and the
proxy adds the real address after it, so the server does not use the addresses the client wrote.
`scheme()` returns `https` when the last value in `X-Forwarded-Proto` is `https`, and only the
response to such a request has the HSTS header. `X-Forwarded-For` is the only address header the
server reads.

```nvs skip
<?nvs
// The proxy at 10.0.0.5 sent `X-Forwarded-For: 192.0.2.99, 203.0.113.7`
// and `X-Forwarded-Proto: https`.
?tainted string $client = Core\Request::clientIp();   // "203.0.113.7"
tainted string $scheme = Core\Request::scheme();      // "https"
```

`clientIp()` returns `null` when the proxy wrote `unknown` in place of the address. A port after
the address is not part of the result. When the address the server would use is not an IP address,
the server answers `400` and runs no program.

**The address that connected is not in the list.** The server reads no forwarding header of that
request, the same as with an empty list.

**A Unix socket.** When the list is not empty, the server also reads the forwarding headers of a
request that arrives over a Unix socket. The operating system decides who may connect to the
socket, so it needs no entry of its own. With an empty list, `clientIp()` returns `null` for such a
request.

A production server whose configuration has a `[server]` block prints one warning at start when
`trusted_proxies` is empty and every address it listens on is a loopback address or a Unix socket.
A server like this is usually behind a proxy, and without the list every request has the address of
the proxy, or none.

<!-- src: `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing` -->

# What reaches a running server

A running server takes a change without a restart, except a change to three keys.

**A code change.** The server checks every file a program has loaded once per `[opcache]
revalidate_freq`, off the request path. When a file changed, the server waits until no file of the
program has changed for `[opcache] settle`: `1s` in production, `100ms` in development. Then it
compiles the whole program again, and the next request runs the new code. A request that is already
running finishes on the code it started with. A file added where the program looks for one, or a
file removed, is a change too. This works in every mode, and a deploy has no step to run after it.

A change that does not compile fails the requests that reach it, with its diagnostic. The server
does not serve the last version that compiled in its place. When the file is fixed, the next check
finds the fix.

**A configuration change.** The server checks its own configuration files every two seconds. A
saved file is applied the same way `nvs ctl reload` applies it: the whole tree is read and checked
first, and only then does the next request see it. A file that does not check is logged once, with
its line, and the running configuration stays. `nvs ctl reload` applies a change at once.

**Three keys need a restart**: `[server] listen`, `[server] socket_mode` and `[server] workers`. A
port below 1024 needs a privilege the server gave up after it opened the socket. `socket_mode` is
applied only when a socket is opened, and `workers` sets how many cores accept requests. A changed one
takes effect at the next start. Until then, the reload names it and logs it once with the running
value and the new one, and `nvs ctl status` lists it. The shipped `nvs.toml` ends each of these
three lines with `restart required`:

```toml
[server]
#listen = ["127.0.0.1:8000"]          # default; restart required
#socket_mode = "0660"                 # default; restart required
#workers = 4                          # default: one per core; restart required

[opcache]
#revalidate_freq = "2s"               # how often the server checks the files a program loaded
#settle = "1s"                        # how long they must stay unchanged before a compile
```

## Deploying

Copying files into place is safe, even when the copy takes a few seconds. The server compiles only
after the program's files have stopped changing for `settle`. If a file changes while it compiles,
it throws that compile away and starts again.

An upload that can pause for longer than `settle`, over a slow link or with a large archive, goes
into a new directory. When the upload is complete, switch a link such as `current` to it. The server
resolves the link at the start of each compile and again at the end, so it never builds one program
from two releases. Under `nvs serve <file>` the next compile after the switch reads the new release,
and static files still come from the release the server started with. A `[[server.mount]]` table
resolves its links when the server starts, so a switch reaches it at the next start.

## What no compiler can check

The compiler checks that the new code agrees with itself. It cannot check data that the old code
wrote and the new code reads: cache entries, sessions, queued jobs and database rows. Requests that
started before a change finish on the old code, so for a short time both versions write. Write the
new code so that it reads what the old code wrote. For example, a job that gains a field needs a
default for the jobs queued without it.

<!-- src: `rule:config/an-edit-reaches-the-next-request-without-a-restart`, `rule:config/a-broken-edit-fails-the-requests-that-resolve-it`, `rule:config/the-config-is-an-immutable-snapshot`, `rule:config/reloadability-is-its-own-field`, `rule:config/a-reload-names-what-it-could-not-apply` -->

# Stopping and reloading: the drain

A stop — `Ctrl-C`, `nvs service stop`, the service manager — and a `nvs ctl reload` both drain.
Nothing new is taken, and a request whose head, body or response is moving is given
`drain_timeout` from the moment its connection sees the drain. Whatever is idle closes at once: a
kept-alive connection between requests, a WebSocket waiting for its peer (closed with `going
away`), an event stream with nothing to write. A program may read the same bit through
`Core\Server::isDraining()`, which is `false` off the server.

<!-- src: `rule:concurrency/a-drain-closes-a-connection-cleanly` -->

# nvs ctl

    nvs ctl reload [--socket <path>]
    nvs ctl config [--socket <path>]
    nvs ctl status [--socket <path>]

`nvs ctl` sends one request to a running server and prints the answer. It reaches the server over
the control socket, which `[control] socket` in the configuration names:

```toml
[control]
socket = "/run/nvs/control.sock"      # on Windows a pipe name: '\\.\pipe\nvs-control'
```

A server has no control socket when the value is `false`, or when no `[control]` block is written.
Only the account that runs the server can use the socket. It has no password and no network
address. The server does not start when another account can write to the directory that contains
the socket. No request runs code of your program.

`nvs ctl` reads the name of the socket from `./nvs.toml`, or from the file that `--config` names.
`--socket` gives the name directly, which selects one server when several run on a host. `nvs ctl`
and the server must be the same version of `nvs`.

**`status`** prints how many requests are running and whether the server is draining.
`config_check` counts the checks that the server made of its own configuration files. A
`restart pending` line follows for each key whose changed value waits for the next start:

```text
$ nvs ctl status
in_flight: 3
draining: false
config_check: 412 passes, 412 stats, 1 paths
restart pending: server.workers (running not written, written 2)
```

**`config`** prints every key that the server is running with, and the file that set it.
`nvs config dump --origin` prints the same list from the files on disk. A difference between the
two lists is a change that the server has not applied.

```text
$ nvs ctl config
control.socket = "/run/nvs/control.sock"    /srv/shop/nvs.toml
limits.memory  = "128M"                     /srv/shop/nvs.toml
mode.default   = "production"               /srv/shop/nvs.toml
```

**`reload`** reads all configuration files again and applies the result at once. It prints an
`applied:` line for each key that it changed. It prints an `ignored:` line for each changed key
that it did not apply, such as a key that needs a restart. `invalidated:` is the number of
compiled program files that the change made stale, and the server compiles them again. A mount
`scan` is expanded again. The server also checks its files by itself every two seconds, and a
change that it already applied is not listed.

```text
$ nvs ctl reload
applied: limits.memory
ignored: server.workers
invalidated: 0
```

When a file has an error, `reload` applies nothing. It prints the error and exits with status `1`,
and the server keeps the configuration it has:

```text
$ nvs ctl reload
error: the server refused `POST /reload` with 409:
error[E0601]: unclosed table, expected `]`
  --> /srv/shop/nvs.toml:7:8
  |
7 | [limits
  |        ^ here
  = note: in `[limits`

note: the running configuration is unchanged
```

A changed `[control] socket` moves the socket without a restart. The new socket answers before the
old one closes.

<!-- src: `rule:config/one-local-control-socket`, `rule:config/a-reload-names-what-it-could-not-apply`, `rule:config/ctl-config-reports-the-live-snapshot`, `rule:config/no-network-control-surface` -->

# nvs service

    nvs service install | uninstall | start | stop | status | run | unit [--config <path>]...

`nvs service install` stores this binary and its arguments with the platform's service manager —
systemd on Linux, the Service Control Manager on Windows — and grants the account it runs as what
it needs, and no more. `uninstall` removes every trace. `start`, `stop` and `status` speak to the
manager; `stop` is answered with a drain, and `status` adds what `nvs ctl status` would say.
`unit` prints the definition that `install` would store, and stores nothing. `run` runs the stored
arguments in the foreground, the way the manager would have started them.

`serve` with no file is accepted by `install` only where the named configuration mounts at least
one entry on disk (`E0630`), because a server with nothing to serve exits at once, and a manager
reports that as a crash loop. The installation chapter has where the binary, the configuration and
the logs go and which folder permissions are checked.
