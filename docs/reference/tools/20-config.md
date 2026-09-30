---
id: config
title: "Configuration: nvs.toml, limits and capabilities"
summary: the `nvs.toml` file — where it is read from, every block the binary accepts, resource limits and their ceilings, capability grants, per-application blocks, includes, secrets, and reading it from a program with `Core\Config`
keywords: nvs.toml, configuration, config, TOML, limits, memory_limit, max_execution_time, limits.hard, ceiling, capabilities, fs.read, fs.write, script.spawn, net.connect, net.listen, net.local, process.exec, cache.shared, capability, permission, sandbox, [[app]], entry, root, include, mode, development, production, ini_get, ini_set, ini_restore, ini_get_all, php.ini, .htaccess, secret, secrets, password_file, /run/secrets, SOPS, sops, age, encrypted secrets, sealed secrets, Vault, LoadCredential, mail, Core\Config
---

# The file and where it is read from

Configuration is one TOML file, `nvs.toml`. There is no `php.ini`, no `.htaccess`, no environment
variable and no command-line switch that sets a directive: what the file says is what is in force.

- `nvs run`, `nvs test` and a bundled executable read `./nvs.toml` from the **working directory**
  — not from the program's directory. No file means an empty configuration, which is complete and
  valid: nothing is limited, nothing is granted.
- `--config <path>` (on any subcommand) names the file to read instead and switches the
  `./nvs.toml` lookup off. Repeat it to read several files in order.
- Every relative path inside a file — an include, a capability root, an `[[app]]` key, a secret
  file — resolves against the directory of the file it is written in.
- A key the binary does not know refuses the run before the program starts (`E0601`), naming the
  block and the keys it expected. A typo can never read as "granted nothing".

```toml file=nvs.toml
[limits]
memory = "256M"

[limits.hard]
memory = "512M"
```
```nvs
<?nvs
echo "memory=", Core\Config::get("memory") ?? "unset", "\n";
echo "ceiling=", Core\Config::get("limits.hard.memory") ?? "unset", "\n";
echo "wall_time=", Core\Config::get("wall_time") ?? "unset", "\n";
```
```output
memory=256M
ceiling=512M
wall_time=unset
```

# The blocks the binary accepts

A directive always lives inside a block; the root table holds none. The full roster the parser
accepts — anything else is `E0601`:

| Block | Holds |
|---|---|
| `[limits]`, `[limits.hard]` | resource limits and their ceilings (below) |
| `[mode]` | `default`, `ceiling` (below) |
| `[capabilities]` | capability grants (below) |
| `[[app]]` | a per-application block: `root` or `entry`, `mode`, `origin`, `[app.limits]`, `[app.limits.hard]`, `[app.capabilities]` (below) |
| `[[include]]` | `path`, `dir`, `optional` (below) |
| `[cache.local]`, `[cache.process]`, `[cache.shared]` | `Core\Cache`'s tiers: `max_size`; `max_size`, `fill_wait`; `url`, `timeout`. The compiled-artifact cache is not here — it is `[opcache]`'s `file_cache_dir` (the `nvs` command chapter) |
| `[db.<name>]` | `driver`, `path`, `host`, `port`, `user`, `password`, `password_file`, `database`, `tls_ca_file`, `statement_cache`, `time_zone`, `slow_query`, `pool` |
| `[db.<name>.pool]` | `max`, `idle`, `lifetime`, `acquire` — the connection pool's bounds, written as a table where `pool = false` turns it off |
| `[db] pool` | `pool = false` written beside the blocks rather than inside one, turning pooling off for every connection this process opens — including one `Core\Db::open` described for itself, which names no block. Bounds are not written here: they belong to the block they size |
| `[mail.<name>]` | `host`, `port`, `from`, `user`, `password`, `password_file`, `timeout` — an SMTP submission endpoint |
| `[log]` | `handler`, `handler_reserve_memory`, `handler_reserve_time`, `target`, `format`, `level` |
| `[http]` | `[http.errors] detail`; `[http.headers]`; `[http.cors]`; `[http.cookies]`; `[http.client]` |
| `[server]`, `[[server.mount]]` | the web server's listen addresses, timeouts and mounts |
| `[debug]`, `[metrics]`, `[trace]`, `[control]`, `[opcache]`, `[deferred]` | their named directives |
| `[[extension]]`, `[[schedule]]` | extension paths; scheduled scripts |

Every accepted key is stored, reported by `nvs config dump`, and readable from a program with
`Core\Config::get`. **What `nvs run` acts on in this build** is a shorter list: `[limits] memory`
is enforced; `[capabilities]`, `[[app]]` and `[[include]]` do what this chapter says; everything
else — the other limits, `[mode]`, `[log]`, `[http]`, `[server]`, `[db]`, `[cache]` — is accepted
and read back, and changes nothing about how a program runs, because the server, database client
and logger that would read them are not in this binary.

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
```
```nvs
<?nvs
echo "2m: ", Core\Config::set("wall_time", "2m") ? "ok" : "refused", "\n";
echo "90: ", Core\Config::set("wall_time", "90") ? "ok" : "refused", "\n";
echo "1m30s: ", Core\Config::set("wall_time", "1m30s") ? "ok" : "refused", "\n";
echo "8x tasks: ", Core\Config::set("max_tasks", "8x") ? "ok" : "refused", "\n";
echo "now ", Core\Config::get("wall_time"), "\n";
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
raise itself to with `Core\Config::set`. A key in `[limits]` with no `[limits.hard]` entry has no
ceiling.

| Key | Unit | Meaning |
|---|---|---|
| `memory` | size | the heap a request may hold |
| `cpu_time` | duration | processor time spent computing, shared by every isolate and task the request spawns; waiting on a database, a socket or a sleep costs none of it |
| `wall_time` | duration | elapsed time from start to finish, waiting included |
| `max_tasks` | count | concurrent tasks |
| `max_output` | size | bytes written to the response |
| `max_regex_steps` | count | how many steps the backtracking regex tier may spend on one subject before it throws; the linear tier runs under no budget |
| `fatal_reserve_memory` | size | the slice of `memory` kept back for the limit handler (`Core\Fatal::onLimit`); not raisable, no ceiling |
| `fatal_reserve_time` | duration | the slice of `cpu_time` kept back for the same handler |
| `max_script_depth` | count | how deep `spawn script` may nest (default 64); not raisable |
| `max_decompressed` | size | the most one `Core\Compress` or `Core\Zip` decompression may produce (default 64M); not raisable, and `false` does not remove it |
| `max_decompression_ratio` | count | the other half of the same bound — output per octet of input (default 1000). A call asks for less through its own arguments and never for more |

`[limits.hard]` takes only the keys a request may raise: `memory`, `cpu_time`, `wall_time`,
`max_tasks`, `max_output` and `max_regex_steps`. Breaching a limit is **not an exception**: nothing
in the program can `catch` it. The request is stopped, the handler registered with
`Core\Fatal::onLimit` runs out of the reserve if there is one, `FATAL: …` is written to standard
error, and `nvs run` exits `1`. What was already written to standard output stays written.

```toml file=nvs.toml
[limits]
memory = "16M"
```
```nvs exit=1
<?nvs
echo "before the cap", "\n";
array<string> $ballast = [];
int $slabs = 0;
while ($slabs < 64) {
    $ballast[] = Core\Str::repeat("x", 1048576);
    $slabs++;
}
echo "never printed", "\n";
```
```output
before the cap
```

**`cpu_time` and `wall_time` answer different questions.** `wall_time` bounds how long a client
waits; `cpu_time` bounds how much of the machine one request burns, so a runaway loop is stopped
while a request parked on a slow query is not. A request cannot compute for longer than it runs, so
`cpu_time` only ever fires when it is set below `wall_time` — set equal, it never does. PHP's one
`max_execution_time` counts CPU time on Linux and elapsed time on Windows: the first is `cpu_time`
here, and PHP-FPM's `request_terminate_timeout` is `wall_time`. A request that needs more — a large
report — raises its own limit with `Core\Config::set("cpu_time", "40s")`, up to `[limits.hard]`,
rather than the starting value being raised for every request. The CPU clock is sampled about twice
a second, so a request may overrun `cpu_time` by up to that much. Linux, macOS and Windows have a
per-thread CPU clock; on a platform without one `cpu_time` is not enforced at all, and the server
says so when it starts. An unset `cpu_time` is no limit.

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
`false`: only the file sets it. Nothing under
`nvs run` behaves differently between the two values in this build — the mode selects defaults for
the logger and the HTTP error pages, and neither is in this binary. `Core\Config::get("mode")`
(without `.default`) answers `null`, and `set("mode", …)` returns `false`.

<!-- primer -->
# `[capabilities]`

A program can do nothing outside its own memory unless the configuration says so. Every effect —
reading a file, writing one, starting another script — is guarded by a named capability, and the
default for every one of them is **denied**: an absent block, a `false`, and an empty list all
deny. The roster is closed:

| Capability | Guards | Scope of a grant |
|---|---|---|
| `fs.read` | `Core\IO::read` and every other read | the directories readable |
| `fs.write` | `Core\IO::write` and every other write | the directories writable |
| `script.spawn` | `spawn script` | the directories a target script may live under |
| `net.connect` | outbound connections | the hosts reachable |
| `net.listen` | binding an endpoint | the `address:port` endpoints bindable, matched exactly; it carries no address policy, and `net.connect` does not imply it |
| `net.local` | connecting to or binding a socket path | the paths or directories reachable; it follows from `net.connect` no more than `net.connect` follows from it |
| `process.exec` | starting a subprocess | the programs runnable |
| `debug.trace`, `debug.profile` | writing a trace or a profile | where it may be written |
| `db.connect`, `db.open` | opening a `[db.<name>]` block; a program-supplied database address | the block names; the hosts |
| `db.schema` | issuing DDL — `Core\Db\Schema::applySafe` and its risky twin | the block names |
| `mail.send` | sending through a `[mail.<name>]` block | the block names |
| `cache.shared` | reaching the store `[cache.shared] url` names — `Core\Cache::shared()` and `Core\RateLimit::consume` | unscoped: a deployment has one shared store |

A capability whose member has not landed yet is still accepted here rather than refused, so a grant
written today keeps meaning the same thing on the build that starts asking for it.

A grant is spelled one of three ways, and a dotted key is the same as a nested block:

```toml
[capabilities.fs]
read = ["data", "/var/app/uploads"]   # these roots, resolved against this file's directory
write = false                         # denied (the same as leaving it out)

[capabilities]
script.spawn = true                   # unrestricted
```

A path is checked after it is canonicalized, then by whole path components: `data/../etc` is not
under `data`, and `data2` is not under `data`. Being able to read a file is not permission to run
it as a script.

**A denial is an ordinary `RuntimeError`**, thrown at the call, naming the member, the capability
and the argument — so a program can catch it and carry on, unlike a limit breach:

```toml file=nvs.toml
[capabilities.fs]
read = ["data"]
```
```txt file=data/greeting.txt
hello from data
```
```nvs
<?nvs
echo Core\Str::trim(Core\IO::read("data/greeting.txt")), "\n";

try {
    Core\IO::write("data/out.txt", "x");
} catch (RuntimeError $denied) {
    echo $denied->message, "\n";
}

try {
    echo Core\IO::read("main.nvs");
} catch (RuntimeError $denied) {
    echo $denied->message, "\n";
}
```
```output
hello from data
Core\IO::write needs the capability `fs.write` for data/out.txt, which is not granted
help: grant it in nvs.toml under `[capabilities.fs]`
Core\IO::read needs the capability `fs.read` for main.nvs, which is not granted
help: grant it in nvs.toml under `[capabilities.fs]`
```

# Network grants: the addresses and endpoints they reach

`net.connect` carries a second key, because a granted host is not automatically a reachable address:
an outbound connection to a loopback, private, link-local or unspecified address is refused whatever
the grant says, since a hostname an attacker influenced can resolve into one. A deployment that must
reach an internal service excepts the address it means, one at a time:

```toml
[capabilities.net]
connect = ["metrics.internal"]   # the names reachable
internal = ["10.4.0.9"]          # the denied addresses this deployment reaches anyway
```

An `internal` entry is an IP address literal — never a hostname, never a range, and `true` is not a
spelling it has. It grants nothing on its own: an address named there is still only reached under a
host `connect` grants.

`net.listen` has no such key, because the policy's terms invert under a bind: binding loopback is
the contained case and binding the unspecified address is the exposed one, so `connect`'s denied
ranges would refuse the safe spelling and admit the dangerous one. An entry is an `address:port`
literal instead, matched as the endpoint it names rather than as the string it was written as, and
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
echo Core\Config::get("memory"), "\n";
echo "300M: ", Core\Config::set("memory", "300M") ? "ok" : "refused", "\n";
echo "200M: ", Core\Config::set("memory", "200M") ? "ok" : "refused", "\n";
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
echo Core\Config::get("memory"), "\n";
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
one of the pair may be set — both is `E0608`. Two directives have such a pair today,
`[db.<name>] password` and `[mail.<name>] password`:

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

`Core\Config::get`, `set`, `restore` and `all` are the program's view of the configuration — the
replacements for `ini_get`, `ini_set`, `ini_restore` and `ini_get_all`.

- A name is the dotted path the file writes: `limits.hard.memory`, `log.level`, `mode.default`. A
  **bare limit name** — `memory`, `cpu_time`, `wall_time`, `max_tasks`, `max_output` — is the
  `[limits]` entry.
- `get` answers the value as text, spelled the way the file spelled it, or `null` for a name
  nothing set and for a name that holds a table or a list (a capability grant, an `[[app]]` block).
- `set` writes an overlay **for this request only**. It is gone when the request ends and is
  invisible to every other request, including a script this one spawns. It returns `false` and
  changes nothing — it never throws — for a `System` directive (only the file may set it), a name no
  directive governs, a value that does not parse in the name's unit, and a value above the
  `[limits.hard]` ceiling. Every directive's class is in the table below.
- `restore` drops the overlay for one name; `all` answers every value in force, keyed by dotted
  name in name order, with the overlay folded over the file.

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

echo "256M: ", Config::set("memory", "256M") ? "ok" : "refused", "\n";
echo "1G: ", Config::set("memory", "1G") ? "ok" : "refused", "\n";
echo "file_cache_dir: ", Config::set("opcache.file_cache_dir", "/tmp/nvs") ? "ok" : "refused", "\n";
echo "unknown: ", Config::set("no.such.key", "1") ? "ok" : "refused", "\n";
Config::set("log.level", "debug");

foreach (Config::all() as string $name => string $value) {
    echo $name, " = ", $value, "\n";
}

Config::restore("memory");
echo "restored: ", Config::get("memory"), "\n";
```
```output
256M: ok
1G: refused
file_cache_dir: refused
unknown: refused
limits.hard.memory = 256M
limits.memory = 256M
log.level = debug
restored: 128M
```

Every directive has a **class** that says who may change it: `Runtime` (the file states a default
and a request may move it up to its ceiling), `RuntimeTighten` (a request may only narrow it),
`System` (the file alone). The `apply` column says whether a running server applies a change to
the file by itself, or only at its next start. Only `[server] listen`, `socket_mode` and `workers`
wait for the next start (the server chapter, § *What reaches a running server*).

<!-- generated: directives -->

# Auditing a tree

`nvs config check` resolves the tree and reports what it holds, exiting non-zero on any refusal;
`nvs config dump` prints every key in force, `--origin` adds which file wrote it and which it
overrode, and `--toml` prints the resolved tree as one document. Both are described with the other
subcommands in the `nvs` command chapter.

```text
$ nvs config check nvs.toml
ok: 1 file, 4 directives set, 0 overrides, 0 warnings
$ nvs config dump nvs.toml
app.0.limits.wall_time = "120s"
app.0.root             = "."
limits.memory          = "256M"
limits.wall_time       = "30s"
```
