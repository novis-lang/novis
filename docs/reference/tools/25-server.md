---
id: server
title: The HTTP server
summary: `nvs serve`, the `[server]` block and its mounts, how a request finds the file that answers it, what a program reads about the door it came through, what reaches a running server, the drain, what happens when the client goes away and `nvs service`
keywords: nvs serve, server, HTTP, disconnect, client disconnect, client goes away, cancel_on_disconnect, disconnect_grace, ignore_user_abort, restart, restart required, deploy, hot reload, zero downtime, symlink, settle, revalidate_freq, listen, port, --listen, --port, [server], [[server.mount]], mount, prefix, host, scan, entry, origin, root, dispatch, static, static files, trusted_proxies, X-Forwarded-For, X-Forwarded-Proto, client IP, health_path, health check, max_in_flight, workers, timeout, drain, drain_timeout, graceful shutdown, reload, nvs service, service, systemd, Windows service, Core\Request::mount, Core\Request\Mount, Core\Router::url, multi-tenant, subdirectory, virtual host, front controller, try_files, php -S, php-fpm, nginx, Apache, .htaccess, RewriteBase, SCRIPT_NAME, PATH_INFO, DocumentRoot
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
`./nvs.toml` and the `nvs.toml` in the data folder (the configuration chapter). With nothing written
anywhere the server listens on `127.0.0.1:8000`.

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
drain_timeout      = "30s"                # after a stop: the longest a connection with no request stays open
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
key is written, the sequence is `try_files $uri /index.nvs`. A prefix is matched exactly, and in steps 3 and 4 the URL has to write the name of
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

**A configuration change.** The server checks its own configuration files every two seconds. When
you save a file, the server reads all of its configuration files again and checks them. Then the next
request uses the new configuration. Every reload writes one `configuration reloaded` line to the log.
Its `applied` field lists the keys that changed. Its `ignored` field lists the changed keys that need
a restart. `invalidated` is the number of compiled program files that the server compiles again.

A file with an error changes nothing. The server writes `configuration reload refused` to the log
once, with the error and its line, and keeps the configuration it has. A saved file is the only
thing that starts a reload. No signal and no command from a service manager starts one.

**Four keys need a restart**: `[server] listen`, `[server] socket_mode`, `[server] workers` and
`[server] watchdog_margin`. A port below 1024 needs a privilege the server gave up after it opened
the socket. `socket_mode` is applied only when a socket is opened, and `workers` sets how many cores
accept requests. `watchdog_margin` is how long a worker may stay past the deadline of its oldest
request before the server reports that the worker has stopped. A changed one takes effect at the
next start. Until then, the server writes `configuration restart pending` to the log once, with the
key, the running value and the new one. The shipped `nvs.toml` ends each of these four lines with
`restart required`:

```toml
[server]
#listen = ["127.0.0.1:8000"]          # default; restart required
#socket_mode = "0660"                 # default; restart required
#workers = 4                          # default: one per core; restart required
#watchdog_margin = "5s"               # default; restart required

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

A stop makes the server drain. A stop is `Ctrl-C`, `nvs service stop` or a stop from the service
manager. The server accepts no new connection, finishes the work it has, and then exits:

- A request that is running is answered in full, and then its connection closes.
- A connection with no work closes at once. This is a kept-alive connection between two requests,
  a WebSocket whose program waits for a message, and an event stream with nothing to write.
- A WebSocket or an event stream whose program keeps writing is closed a short time later.

A WebSocket is closed with the code `1001` (`going away`), so its client can connect again.

`drain_timeout` is the longest time that a connection with no request stays open after it sees
the drain. `false` and `0` are not allowed:

```toml
[server]
drain_timeout = "30s"                 # default
```

While the server drains, `[server] health_path` answers `503`. A program reads the same state with
`Core\Server::isDraining()`, which returns `false` in a program that no server runs. A long request
can test it between two steps and end early:

```nvs skip
<?nvs
array<string> $orders = ['A-1001', 'A-1002', 'A-1003'];
foreach ($orders as string $order) {
    // `true` after the server was told to stop. The loop ends, and the answer is still sent.
    if (Core\Server::isDraining()) {
        break;
    }
    echo 'invoice sent for ', $order, "\n";
}
```

**A reload drains the connections that are open.** When the server applies a saved configuration
file, it closes each connection that was opened before the reload, the same way a stop does. A
request that is running finishes with the configuration it started with, and then its connection
closes. The server still accepts new connections, and they use the new configuration. The health
path still answers `200`, and `Core\Server::isDraining()` still returns `false`.

<!-- src: `rule:concurrency/a-drain-closes-a-connection-cleanly`, `rule:config/the-config-is-an-immutable-snapshot`, `rule:http-server/health-path-is-off-and-checks-nothing` -->

# When the client goes away

A client can close its connection before the answer is sent. For example, the user closes the
browser tab. The request still runs to its end, and the server throws its answer away:

- A write to the answer returns at once and does not throw an error. This is true for `echo`,
  for `Core\Response` and for `Core\Response\Stream::write`. The bytes are thrown away.
- Work that the request gave to `Core\Task::afterResponse` still runs.
- The request keeps its place in `[server] max_in_flight` until it ends.
- When the server drains, it waits for this request as for any other, up to `drain_timeout`.

A request with a `wall_time` stops at its `wall_time`, as it does with a client. A request with no
`wall_time` stops when `disconnect_grace` has passed since the client went away. `false` and `0`
are not allowed:

```toml
[limits]
disconnect_grace = "30s"              # default
```

`cancel_on_disconnect` lists request methods. A request with a listed method stops as soon as its
client goes away. The names are compared exactly, so `GET` and `get` are two different methods:

```toml
[limits]
cancel_on_disconnect = ["GET", "HEAD"]   # default: []
```

A request that stops runs no more code of your program. No `catch` block runs. Both keys can also
be written in `[app.limits]` for one application. A request cannot change them while it runs, and
a program cannot test whether its client has gone. These keys do not apply to an event stream
(`Core\Sse`) or a WebSocket.

<!-- src: `rule:http-server/a-request-outlives-a-client-that-goes-away`, `rule:concurrency/cancellation-runs-no-user-code` -->

# nvs service

    nvs service install <name> [options] -- <nvs arguments>
    nvs service unit <name> [options] -- <nvs arguments>
    nvs service start | stop | status | run | uninstall <name>

`nvs service` installs a server as a service of the operating system. The service manager starts a
service at every boot, and starts it again after a failure. The service manager is systemd on Linux
and the Service Control Manager on Windows.

**`install`** stores one `nvs` command under a name that you choose. Everything before `--` is an
option of the installer. Everything after `--` is the command that the service runs. Run `install`
as root on Linux, and in a terminal started as administrator on Windows:

```text
$ nvs service install shop -- serve /srv/shop/public/index.nvs --config /srv/shop/nvs.toml
installed service `shop`; `nvs service start shop` starts it
```

The installer checks the command first. When a check fails, it prints an error and changes nothing:

| The check | Error |
|---|---|
| The command is `serve` or `run`. No other command keeps running. | `E0630` |
| `serve` with no file needs a configuration with a `[[server.mount]]` entry that is on disk. | `E0630` |
| Every path is a full path. This includes `--config`, `--data`, and `[log] target`, `[opcache] file_cache_dir` and `[io] temp_root` in the configuration. | `E0631` |
| The installer can create and use the data folder, when the service reads the `nvs.toml` in it. | `E0653` |
| On Linux, the account that `--account` names exists. | `E0654` |
| The command line has no `--password`. Other users of the machine can read a command line. | `E0633` |
| A bundled executable does not install itself. Install the `nvs` binary. | `E0634` |

Without `--config`, the service reads the `nvs.toml` in its data folder. The data folder is
`.nvsdata` next to the `nvs` program, or the folder that `--data` names after `--`. The installer
adds `--config` and the full path of that file to the command that it stores. It creates the folder
and the file when they do not exist. A file that already exists is not changed.

The error names what failed the check:

```text
$ nvs service install shop -- serve index.nvs --config /srv/shop/nvs.toml
error[E0631]: `the entry file` is relative: `index.nvs`
  = note: a service does not start in the directory of this terminal, so a relative path names another file there and the service fails at its first start
  = help: write the path absolutely

error: aborting due to 1 error
To read what a code means and how to fix it, run `nvs agent show <code>`.
```

The options of the installer:

| Option | What it sets |
|---|---|
| `--account <account>` | The account that the service runs as. The default is the local system account. |
| `--start automatic`, `delayed` or `manual` | When the service starts after a boot. The default is `automatic`. |
| `--restart on-failure` or `never` | What happens after a failure. The default is `on-failure`. |
| `--depends-on <service>` | A service that must start first, such as a database. You can repeat it. |
| `--description <text>` | The text that an administrator sees beside the name. |
| `--dry-run` | Prints every step and changes nothing. |

With `--account`, the service account can read the data folder and every configuration file. It can
write only to the `cache`, `tmp`, `lsp` and `logs` folders in the data folder, and to a log, cache or
temporary folder that the configuration names. The service cannot change its own configuration.

On Linux, the installer sets owners and modes. The data folder belongs to `root` with mode `0750`,
and its `nvs.toml` with mode `0640`. Both are in the group of the account, so the account can read
them. Each folder that the service writes to belongs to the account, with mode `0700`. The
installer creates such a folder when it does not exist. A `--config` file in another folder is not
changed, and the installer prints a warning when the account cannot read it. `uninstall` gives
every one of these paths back to `root`, readable only by `root`.

A service has no terminal. When the configuration does not set `[log] target`, a service writes its
log to `logs/nvs.log` in the data folder. When that file reaches `[log] max_size`, Novis renames it
to `nvs.log.1` and starts a new file, and it keeps `[log] keep` old files. If the data folder cannot
be used, the log goes to the standard error stream. `nvs service install --help` prints one whole
command line for each platform.

**`unit`** prints what `install` would store, and changes nothing. It makes the same checks, and it
needs no administrator. On Linux it prints a systemd unit. On Windows it prints a `New-Service`
command:

```text
$ nvs service unit shop -- serve /srv/shop/public/index.nvs --config /srv/shop/nvs.toml
[Unit]
Description=Novis service shop
After=network.target

[Service]
Type=notify
ExecStart=/usr/local/bin/nvs serve /srv/shop/public/index.nvs --config /srv/shop/nvs.toml
WatchdogSec=30
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=-/usr/local/bin/.nvsdata/cache -/usr/local/bin/.nvsdata/tmp -/usr/local/bin/.nvsdata/lsp -/usr/local/bin/.nvsdata/logs
PrivateTmp=true
CapabilityBoundingSet=
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
SystemCallFilter=@system-service

[Install]
WantedBy=multi-user.target
```

On Linux, `install` writes this unit to `/etc/systemd/system/shop.service` and enables it. The unit
has no reload command. The server applies a saved configuration file by itself.

`ProtectSystem=strict` makes every file read-only for the service. `ReadWritePaths` lists the folders
that the service can write: the `cache`, `tmp`, `lsp` and `logs` folders in the data folder, and a
log, cache or temporary folder that the configuration names. The `-` before
each path tells systemd to skip a folder that does not exist. If the `nvs` binary, the program or one
of these folders is in `/home`, `/root` or `/run/user`, the unit has `ProtectHome=read-only`. With
`ProtectHome=true` the service could not read or write there.

**`start`** and **`stop`** send the request to the service manager. After a stop the server drains
(§ *Stopping and reloading: the drain*). **`status`** prints the state that the service manager
reports:

```text
$ nvs service status shop
active
```

On Windows the state is a word such as `running` or `stopped`. **`run`** runs the stored command in your
terminal, so you can read what the server prints when it does not start as a service.
**`uninstall`** removes the service and everything that `install` added.

The installation chapter has where the binary, the configuration and the logs go and which folder
permissions are checked.

<!-- src: `rule:packaging/a-service-is-one-stored-argv`, `rule:packaging/the-installer-is-a-sink` -->
