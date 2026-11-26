---
id: config
title: "Configuration: nvs.toml, limits and capabilities"
summary: the `nvs.toml` file — where it is read from, every block the binary accepts, resource limits and their ceilings, capability grants, per-application blocks, includes, secrets, and reading it from a program with `Core\Config`
keywords: nvs.toml, configuration, config, TOML, limits, memory limit, time limit, limits.hard, ceiling, capabilities, fs.read, fs.write, script.spawn, net.connect, net.listen, net.local, process.exec, cache.shared, capability, permission, sandbox, [[app]], entry, root, include, mode, development, production, read a setting, change a setting, .htaccess, secret, secrets, password_file, /run/secrets, SOPS, sops, age, encrypted secrets, sealed secrets, Vault, LoadCredential, mail, SMTP, DKIM, XOAUTH2, OAuth, capture, Core\Config
---

# The file and where it is read from

Configuration is one TOML file, `nvs.toml`. No `.htaccess` file, environment variable or
command-line switch sets a directive: what the file says is what is in force.

- `nvs` reads `./nvs.toml` from the **working directory**. It does not look in the program's
  directory.
- If there is no `./nvs.toml`, `nvs` reads `nvs.toml` in the data folder. The data folder is
  `.nvsdata` next to the `nvs` program, or the folder that `--data` names.
- If neither file exists, the configuration is empty. An empty configuration is complete and valid:
  nothing is limited, and nothing is granted. `nvs run`, `nvs serve`, `nvs test`, `nvs build` and
  `nvs check` then write `nvs.toml` into the data folder, with every key commented out. You can edit
  that file. `--no-init` stops them from writing it. Novis updates this file when you update Novis,
  unless you changed it. After you change it, Novis never touches it again.
- `--config <path>` (on any subcommand) names the file to read instead. Then `nvs` reads neither
  `nvs.toml`. Repeat it to read several files in order.
- A relative path inside a file starts at the folder of that file. This is true for an include, a
  capability folder, an `[[app]]` key and a secret file.
- An unknown key stops the run before the program starts, with the error `E0601`. The error names
  the block and the keys it expected. So a misspelled key always gives an error.

```toml file=nvs.toml
[limits]
memory = "256M"

[limits.hard]
memory = "512M"
```
```nvs
<?nvs
echo 'memory=', Core\Config::get('memory') ?? 'unset', "\n";
echo 'ceiling=', Core\Config::get('limits.hard.memory') ?? 'unset', "\n";
echo 'wall_time=', Core\Config::get('wall_time') ?? 'unset', "\n";
```
```output
memory=256M
ceiling=512M
wall_time=unset
```

# The blocks the binary accepts

A directive always lives inside a block; the root table holds none. The full roster the parser
accepts is below. Any other block is `E0601` when it contains a setting. An unknown block with no
setting under it gives the warning `W1024`, and Novis ignores it.

| Block | Holds |
|---|---|
| `[limits]`, `[limits.hard]` | resource limits and their ceilings (below) |
| `[mode]` | `default`, `ceiling` (below) |
| `[capabilities]` | capability grants (below) |
| `[[app]]` | a per-application block: `root` or `entry`, `mode`, `origin`, `[app.limits]`, `[app.limits.hard]`, `[app.capabilities]` (below) |
| `[[include]]` | `path`, `dir`, `optional` (below) |
| `[cache.memory]`, `[cache.shared]` | `Core\Cache`'s two caches: `max_size`, `fill_wait`; `url`, `timeout`. The compiled-artifact cache is not here — it is `[opcache]`'s `file_cache_dir` (the `nvs` command chapter) |
| `[db.<name>]` | `driver`, `path`, `host`, `port`, `user`, `password`, `password_file`, `database`, `tls_ca_file`, `statement_cache`, `time_zone`, `slow_query`, `journal_mode`, `pool` |
| `[db.<name>.pool]` | `max`, `idle`, `lifetime`, `acquire` — the connection pool's bounds, written as a table where `pool = false` turns it off |
| `[db] pool` | `pool = false` written beside the blocks rather than inside one, turning pooling off for every connection this process opens — including one `Core\Db::open` described for itself, which names no block. Bounds are not written here: they belong to the block they size |
| `[mail.<name>]` | an SMTP server a program sends mail through. `host`, `port`, `from`, `from_name` (the sender's display name), `return_path` (the envelope sender, where bounces go), `timeout`; `user`, `password`, `password_file`; `tls` (`implicit`, `starttls` or `none`), `tls_ca_file`, `auth` (`plain`, `login` or `xoauth2`); `oauth_token_url`, `oauth_client_id`, `oauth_client_secret`, `oauth_client_secret_file`, `oauth_scope`, `oauth_refresh_token`, `oauth_refresh_token_file` for `xoauth2`; `dkim_selector`, `dkim_key_file`, `dkim_domain` to sign each message; `capture` (keep each message and send nothing); `pool` |
| `[mail.<name>.pool]` | `max`, `idle`, `lifetime`, `acquire`: the bounds of the block's kept connections, as `[db.<name>.pool]` has them. `pool = false` opens a new connection for every message |
| `[log]` | `handler`, `handler_reserve_memory`, `handler_reserve_time`, `target`, `max_size`, `keep`, `format`, `level`, `access`, `client_ip` |
| `[regex]` | `backtracking`: `"allow"` (the default), `"warn"` or `"deny"`. It decides what a pattern with lookaround or a backreference does. Under `"warn"`, such a `Core\Regex::compile` literal compiles with a warning. Under `"deny"`, the literal does not compile, and such a pattern built at run time throws a `RuntimeError`. A program cannot change this setting |
| `[http]` | `[http.errors] detail`; `[http.headers]`; `[http.cors]`; `[http.cookies]`; `[http.client]` |
| `[server]`, `[[server.mount]]` | the web server's listen addresses, timeouts and mounts |
| `[debug]`, `[metrics]`, `[trace]`, `[opcache]`, `[deferred]` | their named directives |
| `[[extension]]`, `[[schedule]]` | extension paths; scheduled scripts |
| `[ext.<name>]` | the settings of one extension. The extension's manifest lists the keys and their types. A key it does not list, or a block no loaded extension uses, stops the server from starting |

Every accepted key is stored, reported by `nvs config dump`, and readable from a program with
`Core\Config::get`.

# Values and units

A value is written the way the key's unit reads it, and the *key* decides the unit — `m` is
mebibytes under `memory` and minutes under `wall_time`:

| Unit | Spelling | Bare number means |
|---|---|---|
| size | a whole number with `K`, `M`, `G` or `T` (also `KB`/`KiB`, …); case does not matter | bytes |
| duration | a whole number with `ns`, `us`, `ms`, `s`, `m`, `h` or `d` | seconds |
| count | a whole number, no suffix | — |
| ratio | a number from `0` to `1` | — |

One number, one suffix: `90` and `2m` are durations, `1m30s` and `1.5m` are not. `false` under a
ceiling key means *no ceiling*. A value that does not parse in its key's unit is **not** refused
by `nvs run` or `nvs config check` in this build; it is simply not a quantity anything enforces.

```toml file=nvs.toml
[limits]
wall_time = "30s"

[limits.hard]
wall_time = "5m"
```
```nvs
<?nvs
echo '2m: ', Core\Config::set('wall_time', '2m') ? 'ok' : 'refused', "\n";
echo '90: ', Core\Config::set('wall_time', '90') ? 'ok' : 'refused', "\n";
echo '1m30s: ', Core\Config::set('wall_time', '1m30s') ? 'ok' : 'refused', "\n";
echo '8x tasks: ', Core\Config::set('max_tasks', '8x') ? 'ok' : 'refused', "\n";
echo 'now ', Core\Config::get('wall_time'), "\n";
```
```output
2m: ok
90: ok
1m30s: refused
8x tasks: refused
now 90
```

# `[limits]` and `[limits.hard]`

`[limits]` states what a request **starts with**; `[limits.hard]` states the **ceiling** it may
raise itself to with `Core\Config::set`. When `[limits.hard]` does not set a key, the ceiling is the
value the request started with. So a program can lower that limit, and it cannot raise it. One
exception: a command-line run (`nvs run`, a command, `nvs test`) with no `[limits.hard] memory`
may raise its own `memory` to any value. Server work never goes past `[limits.hard]`.

When `[limits] memory` is not set, every program may hold 128M. When `[limits] cpu_time` is not
set, a request, a connection, a scheduled script and a queued job may use 30s of processor time.
`nvs run` and `nvs test` have no `cpu_time` limit unless you set one. `false` removes a limit.

| Key | Unit | Meaning |
|---|---|---|
| `memory` | size | the heap a request may hold |
| `cpu_time` | duration | processor time spent computing, shared by every isolate and task the request spawns; waiting on a database, a socket or a sleep costs none of it |
| `wall_time` | duration | elapsed time from start to finish, waiting included |
| `max_tasks` | count | concurrent tasks |
| `max_output` | size | bytes written to the response |
| `max_regex_steps` | count | how many steps the backtracking regex tier may spend on one subject before it throws; the linear tier runs under no budget |
| `max_multipart_parts` | count | how many parts a multipart form body may have, text fields included (default 1000, ceiling 10000). A body with more parts makes the call that reads the form throw an error |
| `fatal_reserve_memory` | size | the slice of `memory` kept back for the limit handler (`Core\Fatal::onLimit`); not raisable, no ceiling |
| `fatal_reserve_time` | duration | the slice of `cpu_time` kept back for the same handler |
| `max_script_depth` | count | how deep `spawn script` may nest (default 64); not raisable |
| `max_decompressed` | size | the most one `Core\Compress` or `Core\Zip` decompression may produce (default 64M); not raisable, and `false` does not remove it |
| `max_decompression_ratio` | count | the other half of the same bound — output per octet of input (default 1000). A call asks for less through its own arguments and never for more |
| `memory_high_water` | ratio | a share of `memory`, between `0` and `1`. A request whose memory peak reaches that share writes one `Warn` to the log when it ends, naming the peak, the ceiling and the route. Unset, nothing is written; not raisable |

`[limits.hard]` takes only the keys a request may raise: `memory`, `cpu_time`, `wall_time`,
`max_tasks`, `max_output`, `max_regex_steps` and `max_multipart_parts`. Breaching a limit is **not an exception**: nothing
in the program can `catch` it. The request is stopped, the handler registered with
`Core\Fatal::onLimit` runs out of the reserve if there is one, `FATAL: …` is written to standard
error, and `nvs run` exits `1`. What was already written to standard output stays written.

```toml file=nvs.toml
[limits]
memory = "16M"
```
```nvs exit=1
<?nvs
echo 'before the cap', "\n";
array<string> $ballast = [];
int $slabs = 0;
while ($slabs < 64) {
    $ballast[] = Core\Str::repeat('x', 1048576);
    $slabs++;
}
echo 'never printed', "\n";
```
```output
before the cap
```

**`cpu_time` and `wall_time` answer different questions.** `wall_time` bounds how long a client
waits; `cpu_time` bounds how much of the machine one request burns, so a runaway loop is stopped
while a request parked on a slow query is not. A request cannot compute for longer than it runs, so
`cpu_time` only ever fires when it is set below `wall_time` — set equal, it never does. A request that needs more — a large
report — raises its own limit with `Core\Config::set("cpu_time", "40s")`, up to `[limits.hard]`,
rather than the starting value being raised for every request. The CPU clock is sampled about twice
a second, so a request may overrun `cpu_time` by up to that much. Linux, macOS and Windows have a
per-thread CPU clock; on a platform without one `cpu_time` is not enforced at all, and the server
says so when it starts.

`memory = false` is no limit. A program with no limit can grow until the machine runs out of
memory. A program can also lower its own limit with `Core\Config::set("memory", "64M")`.

In this build **`nvs run` enforces `memory` and `cpu_time`**. `wall_time`, `max_tasks` and
`max_output` are accepted, readable and settable, and a program that exceeds them under `nvs run`
is not stopped.

# `[mode]`

```toml
[mode]
default = "production"     # what an application starts in: "production" or "development"
ceiling = "development"    # the most permissive mode any code may select; unset = the default
```

Both keys are stored and readable: `Core\Config::get("mode.default")` answers what the file set,
and `Core\Config::set("mode.default", "development")` is accepted for the request when `ceiling`
allows `development`, and returns `false` otherwise. `set("mode.ceiling", …)` always returns
`false`: only the file sets it. `Core\Env::mode()` returns `Development` when the file sets
`development`, and `Production` otherwise. `Core\Config::get("mode")`
(without `.default`) answers `null`, and `set("mode", …)` returns `false`.

In `development` mode, a server checks the `Host` header of each request from this machine. A host
it does not know gets `421`. The server chapter's section *Development: the `Host` check* lists the
allowed hosts, and `[server] development_hosts` adds more.

<!-- primer -->
# `[capabilities]`

A capability is a named permission. A program needs one for every effect outside its own memory,
such as reading a file, writing a file or starting another script. Some capabilities have a default
that depends on how the program runs: on the command line or in a server.

Files have their own defaults. A command-line run (`nvs run`, `nvs test`) may read and write every
file. Server work may read the files in its app folder, except `nvs.toml` and files whose name
starts with `.` (such as `.env`). It may write only in the directories `Core\IO::createTemporaryDirectory` made for it.

Outbound connections have a default too. Every run may connect to any public host. A loopback
address (`127.0.0.1`, `::1`) is allowed on a command-line run, and in server work when the
configuration sets `[mode] default = "development"`. Every other private address needs a line.

A command-line run may start any program. A bare name such as `git` is looked up on `PATH`. Server
work starts no program without a line. In `exec = ["git"]` the name is looked up on `PATH` when the
configuration loads, and a name that is not found stops the load. Write a folder with a slash,
`"bin/"`, so it is not read as a program name.

Every run may spawn a script inside its app folder, except below a folder whose name starts with `.`
or a directory `Core\IO::createTemporaryDirectory` made. A script anywhere else needs a `script.spawn` line.

Every run may read environment variables. A package may read them only when its `[grants]` line
lists `"env"`.

Every other capability is **denied** until you write a grant for it. A grant is a key under
`[capabilities]`. A key you write replaces its default, so `false` and an empty list both deny.
This table is the complete list of capabilities:

| Capability | Guards | Scope of a grant |
|---|---|---|
| `fs.read` | `Core\IO::read` and every other read | the directories readable |
| `fs.write` | `Core\IO::write` and every other write | the directories writable |
| `script.spawn` | `spawn script` | the directories a target script may live under |
| `net.connect` | outbound connections | the hosts reachable |
| `net.listen` | binding an endpoint | the `address:port` endpoints bindable, matched exactly. A `net.connect` grant does not give it |
| `net.local` | connecting to or binding a socket path | the paths or directories reachable. `net.connect` and `net.local` are separate grants |
| `process.exec` | starting a subprocess | the programs runnable |
| `debug.trace`, `debug.profile` | writing a trace or a profile | where it may be written |
| `db.connect`, `db.open` | opening a `[db.<name>]` block; a program-supplied database address | the block names; the hosts |
| `db.schema` | changing tables with `Core\Db\Schema::applySafe` and `applyIncludingRisky` | the block names |
| `mail.send` | sending through a `[mail.<name>]` block | the block names |
| `cache.shared` | the store that `[cache.shared] url` names, used by `Core\Cache::shared()` and `Core\RateLimit::consume` | `true` or `false`: a deployment has one shared store |
| `queue.purge` | removing a queue's rows with `Core\Queue::delete` and `purge` | the queue names |
| `env` | reading environment variables with `Core\Env::get`, `secret` and `all`, and `Core\Config::secret` | `true` or `false`; write `env = true` under `[capabilities]` |

Every capability in the table is accepted in `nvs.toml`, also one that no method of this version
checks yet.

A grant is a list or `true` or `false`. A dotted key is the same as a nested block:

```toml
[capabilities.fs]
read = ["data", "/var/app/uploads"]   # these roots, resolved against this file's directory
write = false                         # denied (the same as an empty list)

[capabilities]
script.spawn = true                   # unrestricted
```

Novis first turns a path into its full form, without `..` or links. It then compares whole folder
names. So `data/../etc` is not under `data`, and `data2` is not under `data`. A grant to read a file
does not allow running it as a script.

**A denied call throws an ordinary `RuntimeError`.** Its message names the method, the capability
and the argument. A program can catch it and go on. In this example, the program catches two
denied calls:

```toml file=nvs.toml
[capabilities.fs]
read = ["data"]
write = []
```
```txt file=data/greeting.txt
hello from data
```
```nvs
<?nvs
echo Core\Str::trim(Core\IO::read('data/greeting.txt')), "\n";

try {
    Core\IO::write('data/out.txt', 'x');
} catch (RuntimeError $denied) {
    // The message goes on with the full path of the file. This prints the part before it.
    echo Core\Str::before($denied->message, ' for '), "\n";
}

try {
    echo Core\IO::read('main.nvs');
} catch (RuntimeError $denied) {
    echo Core\Str::before($denied->message, ' for '), "\n";
}
```
```output
hello from data
Core\IO::write needs the capability `fs.write`
Core\IO::read needs the capability `fs.read`
```

# Network grants: the addresses and endpoints they reach

`net.connect` carries a second key, because a granted host is not automatically a reachable address:
an outbound connection to a loopback, private, link-local or unspecified address is refused whatever
the grant says, since a hostname an attacker influenced can resolve into one. A deployment that must
reach an internal service names the addresses or ranges it means:

```toml
[capabilities.net]
connect = ["metrics.internal"]          # the names reachable
internal = ["10.4.0.9", "172.18.0.0/16"] # the denied addresses this deployment reaches anyway
```

An `internal` entry is an IP address or a CIDR range. A hostname is not allowed, `true` is not
allowed, and neither is a range with bits set after its prefix, such as `10.0.0.5/8`. It grants
nothing on its own: an address named there is still only reached under a host `connect` grants.

With no `internal` written, loopback is allowed on a command-line run and in a `development`
configuration. A program that changes its own mode to `development` does not get loopback. A
written `internal` list replaces this default, so it must name loopback too if the program needs it.

`net.listen` has no such key, because the policy's terms invert under a bind: binding loopback is
the contained case and binding the unspecified address is the exposed one, so `connect`'s denied
ranges would refuse the safe spelling and admit the dangerous one. An entry is an `address:port`
written out in full instead, matched as the endpoint it names rather than as the string it was written as, and
one that does not parse as an endpoint matches nothing. `true` is every endpoint this process may
bind.

```toml
[capabilities.net]
listen = ["0.0.0.0:8080", "[::1]:9000"]   # the endpoints bindable
```

Both questions are about an endpoint a *program* names. A store an operator wrote into
`[cache.shared] url` is authorized by that writing, so `cache.shared` grants it and neither the host
nor the address is asked about — which is why a shared store on the loopback needs no `internal`
exception and no `connect` entry.

# Extension grants: what a component may reach

An extension can read files, write files and send HTTP requests only when it is granted that.
Three parties decide, and the extension gets only what all three allow:

1. The `grants` key of the extension's `[[extension]]` entry in `nvs.toml`.
2. The extension's manifest, which lists what the extension asks for.
3. The capabilities of the program that calls the extension, at the moment of the call.

```toml
[[extension]]
path   = "geo.nvsx"
sha256 = "…"
grants = { read = ["data/geo/"], write = ["data/out/"], connect = ["tiles.example.com"] }
```

`read` and `write` are lists of folders. A relative folder starts in the directory of the file it is
written in. The extension sees each folder under its full path. It may read files in a `read`
folder, and read and write files in a `write` folder. `connect` is a list of hosts that the
extension may send HTTP requests to. `grants` has no other keys.

An entry without `grants` gives the extension no files and no hosts. `nvs ext inspect` prints what a
manifest asks for, so you can read it before you write `grants`.

The program's own grants apply too. If the program may read only `data/geo/roads/`, the extension
reads only there, even when `grants` names `data/geo/`. A `write` folder needs both `fs.read` and
`fs.write` in the program's capabilities. A request from the extension is sent like a request from
`Core\Http\Client`, so the address rules of `net.connect`, TLS checks and the proxy apply to it.

A file outside the granted folders, or a host that is not granted, is an error in the extension.
How the extension reports it is up to the extension. A path with `..` or a symbolic link never
leaves a granted folder.

# `[[app]]` — per-application blocks

An application is an entry file path. A `[[app]]` block names either a directory (`root`: every
entry file beneath it) or one file (`entry`), never both and never neither (`E0609`), and carries
the directives for programs it covers:

```toml
[[app]]
root = "."                       # every .nvs under this file's directory
origin = "https://example.test"  # what Core\Router::urlAbsolute prepends

[app.capabilities.script]
spawn = true

[[app]]
entry = "jobs/nightly.nvs"       # one file exactly
mode = "production"

[app.limits]
memory = "128M"

[app.limits.hard]
memory = "256M"

[app.capabilities.script]
spawn = false                    # a block may take a grant away as well as give one
```

When `nvs run <file>` starts, every block whose key covers the file applies, **least specific
first** — the global blocks, then `root` blocks from the widest down, then the `entry` block — and
a later block's value replaces an earlier one's. So a block may narrow a limit, lower a ceiling, or
withdraw a capability the global tree granted. A `root` or `entry` that does not exist refuses the
run rather than silently matching nothing. `[app.limits]` and `[app.limits.hard]` take the same
keys as their global namesakes.

```toml file=nvs.toml
[limits]
memory = "256M"

[limits.hard]
memory = "512M"

[[app]]
entry = "main.nvs"

[app.limits]
memory = "128M"

[app.limits.hard]
memory = "256M"
```
```nvs
<?nvs
echo Core\Config::get('memory'), "\n";
echo '300M: ', Core\Config::set('memory', '300M') ? 'ok' : 'refused', "\n";
echo '200M: ', Core\Config::set('memory', '200M') ? 'ok' : 'refused', "\n";
```
```output
128M
300M: refused
200M: ok
```

# `[[include]]` — a tree of files

A file may pull in others, and the result is one flat stream of assignments in which **the later
assignment wins** — an included file may override the file that included it, and every override is
recorded (`nvs config dump --origin` prints both sides).

```toml
[[include]]
path = "conf.d/local.toml"   # one file, relative to this file's directory

[[include]]
dir = "conf.d"               # every *.toml directly inside, in filename order; not recursive

[[include]]
path = "secrets.toml"
optional = true              # absent is fine; unreadable or unparseable is not
```

An include that names a missing file without `optional = true` is `E0605`; a cycle is `E0606`.
Array-of-tables blocks (`[[app]]`, `[[include]]`, `[[schedule]]`, `[[extension]]`) accumulate
across the tree; a plain value is replaced.

```toml file=nvs.toml
[limits]
memory = "128M"

[[include]]
path = "conf.d/local.toml"
```
```toml file=conf.d/local.toml
[limits]
memory = "256M"
```
```nvs
<?nvs
echo Core\Config::get('memory'), "\n";
```
```output
256M
```

# Trust

A configuration file grants capabilities, so it is part of the program's trust boundary. The rule
is that no file the tree reads — nor the directory holding it, nor a secret file it names — may be
writable by any account other than the one the runtime runs as; a file that fails this is refused
with `E0607`. The check belongs to a served host reading its tree as the serving account. In this
build neither `nvs run` nor `nvs config check` applies it: `nvs run` executes a program the same
person chose, from a working directory they chose, and an offline audit on another machine cannot
answer the question the check asks.

A tree that passes on Linux. Only `root` can write, and the group `novis` that runs the server can
read:

```text
drwxr-x---  root  novis  /opt/novis/config/
-rw-r-----  root  novis  /opt/novis/config/nvs.toml
drwxr-x---  root  novis  /opt/novis/config/conf.d/
-rw-r-----  root  novis  /opt/novis/config/conf.d/10-limits.toml
```

`chmod 0664 /opt/novis/config/nvs.toml` gives the group the right to write, and the next start
fails with `E0607`.

[Installing on a host](#tools-install) lists the permissions that pass, the commands that set them
on Windows and on Linux, and what each refusal message means.

# Secrets from files

A directive that holds a secret has a `_file` sibling: the file's **whole content** is the value,
with exactly one trailing newline (and a `\r` before it) removed and nothing else trimmed. Exactly
one of the pair may be set — both is `E0608`. These directives have such a pair:
`[db.<name>] password`, `[mail.<name>] password`, `[mail.<name>] oauth_client_secret`,
`[mail.<name>] oauth_refresh_token`, `[ldap.<name>] password`, `[http.client.proxy] password`,
`[http] csrf_key` and `[cache.shared] password`:

```toml
[db.main]
driver = "postgres"
host = "db.internal"
password_file = "/run/secrets/db-password"

[mail.relay]
host = "smtp.internal"
from = "app@example.test"
user = "app"
password_file = "/run/secrets/mail-password"
```

The value is read at boot, and again each time the configuration is applied, never per request. It never appears
in a diagnostic or a dump: `nvs config dump` prints `<secret>` and names the file the value came
from, so an audit can act on the file without the credential passing through the audit.
`Core\Config::get("db.main.password")` returns the value. `Core\Config::get("db.main.password_file")`
returns the path as it was written, so a program can say where a credential came from without
printing it.

A secret file is part of the trust boundary above, because an account that can rewrite it chooses
the credential the server connects with. So a secret file another account can **write** refuses the
boot; one another account can only **read** is a warning and not a refusal, because a Docker Compose
secret is mounted `0444` and a Kubernetes secret volume defaults to `0644`, and from inside a
container that is the norm rather than a mistake.

A credential is used exactly as it was written, and no part of the runtime trims one: a password may
legitimately begin or end with a space. Because such a space is invisible — there is nothing to see
inside a secret file, and at the end of a TOML line it is one character before a quote — the boot
warns `W1007` naming the directive, and goes on with the value.

There is no `${ENV_VAR}` interpolation and no `--set`. A directive's value is written in the file,
or it is the content of a file the directive names.

## Encrypted secrets: SOPS, `age`, sealed secrets

Novis decrypts nothing itself, and does not need to: every secret-management tool in common use ends
by producing a plaintext file, and `_file` is the seam that takes one.

| How the deployment manages secrets | What it produces | What `nvs.toml` names |
|---|---|---|
| [SOPS](https://getsops.io/) with `age` or a KMS, in a task runner or a `make` target | `sops -d --extract '["db"]["password"]' secrets.enc.yaml > /run/novis/db-password` | `password_file = "/run/novis/db-password"` |
| SOPS under systemd | a `sops -d` in `ExecStartPre`, or `LoadCredential=` | `password_file = "/run/credentials/novis.service/db-password"` |
| Docker Compose | a `secrets:` entry, mounted `0444` | `password_file = "/run/secrets/db-password"` |
| Kubernetes — Flux's kustomize-sops, the SOPS operator, Sealed Secrets, an external-secrets sync | a `Secret`, mounted as a volume | `password_file = "/etc/secrets/db-password"` |
| Vault or a cloud secret manager | an agent or sidecar templating a file | `password_file = "/run/novis/db-password"` |

Two things make this work rather than merely parse. Write the decrypted file where only the serving
account can read it — a `tmpfs` mount, or the directory systemd's `LoadCredential=` hands the unit —
because the boot warns about one the rest of the host can read. And decrypt **before** the server
starts: a `password_file` that is not there yet is a boot refusal naming the path, which is the
failure an operator wants, rather than a first request that cannot connect.

This keeps the decryption key out of the serving process. The process holding the master key and the
process serving requests are deliberately different processes, and a file path is what separates
them.

# Reading and changing it from a program: `Core\Config`

`Core\Config::get`, `set`, `restore` and `all` are the program's view of the configuration.

- A name is the dotted path the file writes: `limits.hard.memory`, `log.level`, `mode.default`.
  A limit's name alone, such as `memory`, `cpu_time` or `wall_time`, is the setting in `[limits]`.
- `get` returns the current value of a setting as text. A value from the file is written exactly
  as the file writes it. A setting the file does not write gives its built-in default, such as
  `128M` or `30s`.
- `get` returns `null` in three cases. The first is a setting with no default, such as `wall_time`.
  The second is a setting that is not written, such as `[log] target`. The third is a name whose
  value is a table or a list.
- `set` changes a setting **for this request only**. The change ends with the request. No other
  request sees it, and a script this request starts does not see it either.
- `set` returns `false` and changes nothing when the change is not allowed. That is a setting a
  request may not change, or an unknown name. It is also a value that is not valid for the setting,
  or a value above its ceiling in `[limits.hard]`. `set` never throws an error.
- `restore` deletes this request's change to one setting. The setting has the file's value or its
  default again.
- `all` returns every current value as an array, with the dotted names as keys, in name order. The
  example below prints three of them.

```toml file=nvs.toml
[limits]
memory = "128M"

[limits.hard]
memory = "256M"

[log]
level = "info"
```
```nvs
<?nvs
use Core\Config;

echo '256M: ', Config::set('memory', '256M') ? 'ok' : 'not allowed', "\n";
echo '1G: ', Config::set('memory', '1G') ? 'ok' : 'not allowed', "\n";
// The compiler checks a setting name written as text in `set`. These two names would not
// compile there. Here they are in variables, so each call runs and returns `false`.
string $cache = 'opcache.file_cache_dir';
string $unknown = 'no.such.key';
echo 'file_cache_dir: ', Config::set($cache, '/tmp/nvs') ? 'ok' : 'not allowed', "\n";
echo 'unknown: ', Config::set($unknown, '1') ? 'ok' : 'not allowed', "\n";
Config::set('log.level', 'debug');

foreach (Config::all() as string $name => string $value) {
    if ($name == 'limits.memory' || $name == 'log.level' || $name == 'session.short_lived') {
        echo $name, ' = ', $value, "\n";
    }
}

Config::restore('memory');
echo 'restored: ', Config::get('memory'), "\n";
```
```output
256M: ok
1G: not allowed
file_cache_dir: not allowed
unknown: not allowed
limits.memory = 256M
log.level = debug
session.short_lived = 2h
restored: 128M
```

The file gives the value each request starts with. A request may change only the keys in the
second table below, and each row says how. `Core\Config::set` returns `false` for every other key.
The compiler checks a name written as text in the call. If the name is not in the table, the
program does not compile.

The `apply` column says whether a running server applies a change to the file by itself, or only
at its next start. Only `[server] listen`, `socket_mode`, `workers` and `watchdog_margin` wait for
the next start (the server chapter, § *What reaches a running server*).

<!-- generated: directives -->

<!-- generated: settable -->

# Auditing a tree

`nvs config check` reads the configuration and reports what it contains. Its exit status is not `0`
when the configuration has an error. `nvs config dump` prints every key in force. That is each key a
file sets, and the default value of every other key. `--origin` adds the file that set each key and
the file it replaced. A default value shows `default`, or `default (production)` when the mode chose
it. `--toml` prints the keys the files set as one TOML document. The `nvs` command chapter describes
both commands with the other subcommands.

```text
$ nvs config check nvs.toml
ok: 1 file, 4 directives set, 0 overrides, 0 warnings
$ nvs config dump nvs.toml
app.0.limits.wall_time = "120s"
app.0.root             = "."
limits.memory          = "256M"
limits.wall_time       = "30s"
log.target             = "stderr"
```

The example shows the four keys that the file sets and one default. The real list has every
default.
