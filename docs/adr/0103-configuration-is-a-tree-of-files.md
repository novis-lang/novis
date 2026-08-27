# ADR 0103 — Configuration is a tree of files, and file ownership is the trust anchor

- **Status:** Accepted
- **Date:** 2026-08-27
- **Scope:** how the running configuration is assembled — where the root file is found, `[[include]]`,
  the order in which files are merged, what a relative path means, the ownership check every config
  input must pass, how a secret reaches a directive, which CLI flags may override one, and the
  `nvs config` verbs that make the result auditable. It does **not** decide the file's *syntax*, which
  stays [0064](0064-configuration-file-format.md)'s, nor the directive registry, changeability classes
  or the per-request overlay, which stay [0005](0005-config-changeability.md)'s, nor what an
  application *is*, which is [0104](0104-an-application-is-an-entry-file-path.md)'s.
- **Amends:** [0064](0064-configuration-file-format.md) — § 1 gains the include tree and the resolution
  order; § 3's duplicate-key refusal gains its cross-file boundary; § 4's "a path the operator hands the
  host" becomes the four-step resolution of § 1 below, `./nvs.toml` included.
  [0078](0078-config-reload-and-control-socket.md) § 3 — `nvs ctl reload` is no longer the only
  operation; reload re-walks the whole tree and re-runs § 6's checks.
  [0067](0067-core-db.md) § 4 — `password` gains a `password_file` sibling.
  [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) § 3 and
  [0097](0097-development-server-and-proxied-origin.md) § 4 — their "ordinary CLI precedence" claims
  become § 8's one rule.
  [0093](0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md) § 2 — `config` joins the
  namespace list and the non-hostable subcommands, and an install whose config comes from the working
  directory is refused.

> **In short:** the configuration is a **tree of TOML files**, not one file. A root file is named by
> `--config` (repeatable) or found as `./nvs.toml`, and pulls in more with `[[include]]`, by `path` or by
> `dir`. Resolution is one ordered stream — each file's own keys, then its includes, depth-first — and
> **later wins**, so an include overrides the file that pulled it in and every override is recorded with
> both origins. A duplicate key *inside* one file is still refused; across files it is the point.
> **Every file in the tree is trusted exactly as far as its ownership**: a config input writable by any
> account but its owner refuses the boot, and because an `optional = true` include that is absent has no
> file to check, the check falls on the directory that would hold it — otherwise "missing" is a standing
> slot anyone able to write that directory may fill with capability grants. A secret arrives as a file
> whose whole content is the value. CLI flags that set a directive are a **closed list** at the global
> layer; there is no `--set`, because argv is world-readable and would be a second spelling of the whole
> format.

## Context

- [0064](0064-configuration-file-format.md) decided the *syntax* and assumed one file at "a path the
  operator hands the host". Every deployment shape that has come up since needs more than one: a base
  plus a per-environment overlay, a host-local file that may or may not exist, a distro dropping in a
  fragment without editing the operator's file, a per-host database block. With one file, each of those
  is solved by templating the whole file in Ansible or Nix — which works, and which means the file the
  operator reads is not the file the server reads.
- **The rule 0064 § 3 actually protects was never "no overriding".** It refuses a duplicate key because
  "a line silently overridden by a later copy of itself is a security-relevant failure that costs nothing
  to refuse". Inside one file a duplicate can only be a mistake. Across an `[[include]]` line the
  operator typed, an override is the stated intent — so the property to preserve is not *no override*
  but *no invisible override*, and that is a reporting obligation rather than a refusal.
- **"Root-owned `nvs.toml`" was an assumption nothing checked.** The phrase carries real weight in about
  fifteen ADRs: it is why an operator-written database address is pre-approved
  ([0067](0067-core-db.md) § 6), why an extension hash pin is worth anything
  ([0003](0003-extension-system.md)), why a capability grant means what it says. Meanwhile the *cache*
  directory ([0042](0042-on-disk-artifact-cache-format.md) § 5) and the *control socket* directory
  ([0078](0078-config-reload-and-control-socket.md) § 3) each get a startup refusal. The file that
  grants `process.exec` had less protection than the socket used to reload it.
- A tree makes that gap load-bearing rather than latent. Once one file can pull in another, "which files
  am I trusting, and why" has to have an answer before the feature exists, not after.
- **Secrets arrive as files now.** Docker Compose secrets, Kubernetes secret volumes and systemd's
  `LoadCredential=` all deliver a credential as a file whose content is the value. A `password` written
  inline is not wrong — it is the same bar as `.pgpass` — but it forces a deployment to either bake the
  secret into an image or bind-mount an entire config file to inject one string.

## Decision

### 1. The root of the tree: `--config`, else `./nvs.toml`, else the shipped defaults

Four steps, first hit wins:

1. Every `--config <path>`, in the order given. The flag is **repeatable**, and the files form an ordered
   list resolved by § 3 exactly as an include list is. A `--config` naming a file that does not exist is
   always a hard refusal — optionality is a property a file declares about *its* includes (§ 6), never
   something argv can assert.
2. `./nvs.toml` — **exactly one directory, never a walk upward.** Any explicit `--config` disables this
   step entirely, so an operator naming files never gets a surprise merge with whatever is in the
   working directory.
3. Otherwise the shipped defaults, which are a complete and valid configuration: capabilities deny-all,
   `[mode] default = "production"`.

There is no platform path, no build-time path and no lookup beside the binary; § *Alternatives rejected*
argues each. Whatever wins is **canonicalized to an absolute path once, at boot, and stored** — `nvs ctl
reload` re-reads that stored path and never re-resolves the working directory, so a deployment that
replaced its directory under a running process reloads the file the operator can see rather than an
unlinked inode. The resolved path and every file the tree reached are printed at boot:

```console
$ nvs serve
info: configuration ./nvs.toml -> /srv/www/app/nvs.toml
info: resolved from 5 files
info:   /srv/www/app/nvs.toml
info:   /srv/www/app/conf.d/10-limits.toml
info:   /srv/www/app/conf.d/20-db.toml
info:   /etc/nvs/local.toml
```

**`./nvs.toml` is the part of this that [0064 § 4](0064-configuration-file-format.md) previously
forbade**, and the reason it moves is § 6 rather than convenience. 0064's objection was a configuration
file *discovered* near a document root, in a directory the serving account can write — which is the
confused-deputy shape PHP spent two decades on with `.htaccess` and `.user.ini`. Three things close it
here and none of them existed when 0064 was written: the ownership check refuses a file any other account
can write; the resolved absolute path is announced, so reading the wrong file is visible in one line
rather than silent; and
[0093](0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md)'s installer refuses to store a
service whose config came from the working directory, so production always carries an absolute
`--config`. What survives is one narrow case — an interactively-started `nvs serve` in a directory the
runtime account can write — and it is stated as a consequence below rather than defended.

### 2. `[[include]]` takes a `path` or a `dir`

Top-level only, and an array-of-tables for the reason [0064 § 2](0064-configuration-file-format.md) gives
for `[[extension]]`: a repeated record with more than one field.

```toml
[[include]]
path = "conf.d/production.toml"

[[include]]
dir = "conf.d"                    # every *.toml directly inside, ascending by filename

[[include]]
path = "/etc/nvs/local.toml"
optional = true                   # absent is fine; see § 6
```

An entry carries `path` **or** `dir`, never both and never neither. `dir` is **not** a glob: it reads
every `*.toml` directly in that directory, in ascending byte order of filename, without recursing. The
order is mandated rather than inherited from `readdir` because § 3 makes order decide the answer, and a
capability grant settled by directory-entry order is not a design. A pattern language is rejected below.

An `[[include]]` inside an included file is ordinary; a cycle is refused with the chain named, and depth
is capped at 8. An included file is a config file in every other respect: unknown keys and duplicate keys
are refused inside it exactly as [0064 § 3](0064-configuration-file-format.md) refuses them in the root.

### 3. One ordered stream, and later wins

The tree flattens to a single sequence: for each `--config` file in order, that file's own keys, then its
includes depth-first in list order; then the next `--config` file. **A later assignment wins**, which
makes an include override the file that pulled it in — the base-plus-local shape, with the include line
at the point the operator wants overridden.

`[0064 § 3](0064-configuration-file-format.md)`'s refusal keeps its original scope: **a duplicate key
within one file is still an error.** What replaces it across files is an obligation, not a permission —
every override is recorded with both origins, surfaced in the boot log and in full by `nvs config dump
--origin` (§ 9). This is the whole of what makes "later wins" acceptable in a file that grants
capabilities; without the record it is the silent-shadowing failure 0064 refused INI for, and it should
be read as a condition on this section rather than as a separate feature.

### 4. A value array replaces; a `[[table]]` appends

`key = [...]` is one value, and § 3 says a later value wins — so `[debug] mode`,
`[capabilities] script.spawn` and `debug.trace` are **replaced wholesale** by a later file. `[[extension]]`,
`[[schedule]]`, `[[server.mount]]`, `[[app]]` and `[[include]]` entries **accumulate**.

The split is not a special case: it is what each shape already means inside one file. Two `[[schedule]]`
blocks in one file are two schedules, so appending across files extends a rule rather than adding one,
and replacing would make the cross-file behaviour differ from the within-file behaviour of the same
syntax. For a capability the direction also matters on its own: replacement means **the last file that
mentions a grant states the whole grant**, and no reader has to assemble the effective root list from
four files to know what it is.

### 5. A relative path resolves against the file it is written in

Every path-valued directive — `cache.dir`, `capabilities.script.spawn`, `debug.trace`,
`[[extension]] path`, `[[server.mount]] root`, `[db.<name>] path`, and `[[include]]`'s own `path`/`dir` —
resolves relative to the directory of the file the value appears in. A path given on the *command line*
resolves against the working directory, because that is what a shell argument means.

```toml
# /etc/nvs/nvs.toml
[[include]]
path = "conf.d/db.toml"            # -> /etc/nvs/conf.d/db.toml

# /etc/nvs/conf.d/db.toml
[db.main]
path = "data/app.sqlite"           # -> /etc/nvs/conf.d/data/app.sqlite
```

One rule shared with `[[include]]`, and the only one under which a config directory survives being copied
or relocated whole. The resolved absolute path is what the boot log and `nvs config dump` print, so the
rule never has to be applied in a reader's head.

### 6. Ownership is the trust boundary, and `optional` moves the check to the directory

**Every file the configuration reads must be owned by the account the runtime runs as or by root, and
must not be group- or world-writable.** Its containing directory must not be group- or world-writable
either. This is the same check [0042 § 5](0042-on-disk-artifact-cache-format.md) applies to the cache
directory and [0078 § 3](0078-config-reload-and-control-socket.md) to the socket directory, applied where
it matters most; a failure is a refusal to start, naming the path and the mode, and it is re-run on every
`nvs ctl reload`. On Windows the equivalent is an ACL check, and which ACEs it accepts is M6's to state.

Because that boundary is uniform, **any file in the tree may set any directive**, `System` class
included: capabilities, `[limits.hard]`, `[mode] ceiling` and `[[extension]]` entries are as legitimate in
`conf.d/host.toml` as in the root file. Whoever can write an included file cleared exactly the same bar as
whoever can write the root file, so a rule restricting what an include may say would buy no security an
attacker does not already have, while costing per-host capability sets and per-environment extension sets.

**`optional = true` covers absence and nothing else.** A file that exists but cannot be read, that fails
the ownership check, or that fails to parse is a hard refusal — otherwise a stray `chmod` silently drops
half a configuration and the server comes up looking healthy. And because an absent file offers nothing to
check, **the check falls on the directory that would contain it**:

```console
E06xx: [[include]] /etc/nvs/local.toml is optional, but /etc/nvs is group-writable
       (mode 0775, group `deploy`)
       an absent optional include is a standing slot that anyone able to write that
       directory may later fill with root-owned configuration
       fix: chmod 0755 /etc/nvs
```

That refusal is the whole security argument for the feature. An optional include is a promise that a file
which does not exist yet will be trusted when it appears; the only place to make that promise safely is
the directory, and it is the one place a check can still run.

### 7. A secret arrives as a file whose content is the value

A directive the registry marks **secret** — `[db.<name>] password` today, and whatever joins it — gains a
`_file` sibling. Exactly one of the pair may be set; both is a refusal, so this is two sources for one
value rather than the second spelling [0015](0015-no-name-aliasing.md) refuses.

```toml
[db.main]
driver = "postgres"
password_file = "/run/secrets/db_password"
```

**The file's entire content is the value**, with exactly one trailing `\n` — and a `\r` immediately before
it — stripped if present, and nothing else trimmed. Not a general trim: a password may legitimately begin
or end with a space, and removing it is the [0095](0095-ambiguous-input-is-refused-never-repaired.md)
failure of repairing input instead of reading it. One newline goes because `echo secret > f` produces one
and every injection system does; a value that genuinely ends in a newline is written with two. The content
must be valid UTF-8 for a string-typed directive, and an empty file, a whitespace-only file, or one over
64 KiB is refused — an empty credential otherwise fails at the first request rather than at boot, and the
size cap catches a path pointed at the wrong thing.

The permission rule splits, because the platforms disagree with the strict answer: **group- or
world-writable is a refusal** like any config input, since another account able to rewrite the file
chooses the credential the server connects with. Group- or world-*readable* is a `W1xxx` warning naming the
mode, not a refusal, because Docker Compose mounts secrets `0444` and Kubernetes secret volumes default to
`0644` — inside a container that is the norm, on a shared host it is not, and Novis cannot tell which it is
in. Integrity is enforced; confidentiality is advised.

The value is never logged and never printed: `nvs config dump` renders it `<secret>` and names the file it
came from, which is [0033](0033-secret-qualifier-for-confidential-values.md)'s type-level meaning applied
one layer below the language.

### 8. A CLI flag is a closed list, at the global layer

[0091 § 3](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) and
[0097 § 4](0097-development-server-and-proxied-origin.md) each assert "ordinary CLI precedence" for their
own flag. Stated once: **a flag that maps to a directive wins over every file, and the set of such flags
is closed** — `--mode`, `--listen`, `--port`, and whatever a later ADR argues for where it introduces the
directive.

**There is no `--set <key>=<value>`.** It would be a second spelling for every directive in the format,
which is [0064](0064-configuration-file-format.md)'s reason for refusing to accept two file formats one
layer up; it would put values into argv, which is world-readable through `ps` and `/proc/*/cmdline`, so a
secret could be set there in a way § 6's file-ownership model has no equivalent of; and the ergonomic case
for it mostly disappeared at § 1, where every project directory acquired a `./nvs.toml` for free.

A flag **replaces the global value** and per-app blocks still layer over it
([0104](0104-an-application-is-an-entry-file-path.md)). The whole stack, outermost first:

```
shipped defaults
 -> the file tree      (--config list + includes, later wins)
   -> CLI flags        (closed list, replaces the global value)
     -> [[app]] blocks (all that match, least-specific root first)
       -> Core\Config::set  (per-request overlay, ADR 0005, unchanged)
```

So `nvs serve --mode=development` on a mixed host does not drag an application that pins `production`
along with it, and `[mode] ceiling` still bounds what any of them may select.

### 9. `nvs config check`, `nvs config dump`, `nvs ctl config`

§ 3's precedence is only safe while it is auditable, so the reporting is part of this decision rather than
tooling around it. `config` is a namespace beside `ctl` and `service`
([0093 § 2](0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md)), and stays out of
`nvs check`, which checks source.

```console
$ nvs config check --config /etc/nvs/nvs.toml
ok: 5 files, 47 directives set, 3 overrides, 1 warning

$ nvs config dump --origin
limits.memory             = "512M"    prod.toml:4   (overrides base.toml:2)
limits.hard.memory        = "2G"      base.toml:9
capabilities.process.exec = true      conf.d/host.toml:3
db.main.password          = <secret>  /run/secrets/db_password (conf.d/20-db.toml:5)

$ nvs config dump --toml > effective.toml     # one canonical file, for diffing environments
$ nvs ctl config --origin                     # what the running process actually holds
```

`check` and `dump` are offline and need no server, so a config tree is validated in CI before it is
deployed. `ctl config` answers the question the offline pair cannot — what a reload actually published,
including an `optional` include that has appeared since boot — and is the second `ctl` operation,
[0078 § 3](0078-config-reload-and-control-socket.md) having reserved the socket for exactly this kind of
read.

## Consequences

- **Cost, as [0004](0004-memory-for-simplicity.md) requires it be stated: none on the request path.** The
  whole tree is parsed at boot and on `nvs ctl reload`, over files measured in kilobytes. What it adds to
  a live snapshot is the origin map — one `(file, line)` per directive set, plus the overrides — which is
  bounded by the number of directives, not by traffic, and is what § 9 reads.
- **The interactive `nvs serve` in a writable directory is a real remaining hole.** If the account the
  runtime runs as can write the directory it was started in, that account can write the `./nvs.toml` it
  reads. § 6's check passes, because the file's owner *is* the runtime account. The service path is where
  production lives and 0093's installer closes it there; on a developer's machine the account already owns
  the process. This is stated rather than defended: an operator who wants it closed passes `--config`.
- **A correct configuration can now fail to boot because of a directory's mode.** That is the intended
  behaviour of § 6 and it will be somebody's confusing afternoon, which is why the diagnostic names the
  path, the mode, the group and the fix.
- **`nvs.toml` becomes a filename that appears in project directories**, where [0081](0081-packages-are-digests-resolution-is-a-maximum.md)
  forbids one inside a *package*. That refusal is unchanged and unaffected — a package is not a working
  directory — but the social pressure to commit a `nvs.toml` is new, and the answer is that committing a
  development config is fine and deploying it is what `--config` is for.
- **Two rules where there was one**, in two places: duplicate keys (within a file versus across files) and
  arrays (values versus `[[tables]]`). Both are the same shape — the cross-file rule extends what the
  syntax already means within a file — but a reader meets them as exceptions.

## Alternatives rejected

- **A platform default path (`/etc/nvs/nvs.toml`).** The conventional answer, and it loses to `./nvs.toml`
  only because two implicit lookups are worse than one; the deployments that want a fixed path are exactly
  the ones running under a service manager, which carries `--config` anyway.
- **A config path baked in at build time**, nginx's `--conf-path`. Genuinely good, and rejected for the
  same reason: a third resolution step for a case `--config` already covers.
- **A lookup beside the binary.** Fixes the instability of a working directory, at the cost of putting
  configuration under `/usr`, where a package upgrade replaces it and neither dpkg's conffile handling nor
  rpm's `.rpmnew` applies; one binary serving several deployments then shares one implicit file; and
  "the path of the running binary" is three platform APIs, a symlink policy, and a `(deleted)` case after
  an upgrade under a live process. [0048 § 1](0048-portable-single-file-executables.md) had already closed
  the strongest argument for it by refusing to bundle a serving deployment at all.
- **Walking up from the entry file to find a manifest.** [0061](0061-compile-time-autoload-and-program-discovery.md)
  rejected it and that rejection stands; § 1's single-directory `./nvs.toml` is deliberately one step short
  of it, and the step is not taken.
- **Refusing any cross-file duplicate — an include may only add keys no other file set.** Preserves
  0064 § 3 untouched and is the safest thing here, but it removes the reason to split files: a host-local
  file could never override a base default, only fill a hole the base deliberately left. § 3's reporting
  obligation is what buys the capability back at an acceptable price.
- **`System`-class directives confined to the root file.** Attractive until § 6 makes every file in the
  tree equally hard to write, at which point it protects nothing and blocks per-host capabilities.
- **An include that may only narrow capabilities**, mirroring [0005](0005-config-changeability.md)'s
  `RuntimeTighten`. It inverts deny-by-default: the root file would have to grant every right any
  environment needs so that each host could take some away.
- **Glob patterns in `path`.** A grammar to specify, fuzz and diagnose — `*.conf` versus `*.toml`, `**`
  and its recursion order, character classes — for a feature whose whole job is "read this directory".
  `dir` is the same capability with one knob.
- **`${ENV_VAR}` interpolation.** The general answer to secrets and to per-environment values alike, and
  it reopens the door 0064 closed against Dhall: a configuration file that computes. [0091 § 3](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)
  already refuses to read the run mode from the environment on purpose, and an inherited, unaudited
  namespace is not where the file that grants capabilities should get its values.
- **A general `*_file` suffix for every string directive.** One uniform rule, at the cost of doubling the
  key space of the format and complicating `deny_unknown_fields`, to serve a handful of directives that
  the registry can mark instead.
- **A `sha256` pin on an `[[include]]`,** matching `[[extension]]`. The pin exists on an extension because
  it accepts a precompiled binary from outside the deployment; an include is a text file under the same
  ownership check, edited by hand, and a pin on it is friction that would be stale by the next edit.
  Reopen it if an include ever arrives from a package.

## Revisiting

Reopen § 1 if `./nvs.toml` turns out to be read in production deployments in practice — the boot line
naming the resolved path is what would show it, and the answer would be to confine the working-directory
step to the terminal-attached subcommands rather than to remove it. Reopen § 4's split if a third array
shape appears that fits neither half.

## Verification

In M6, alongside [0064](0064-configuration-file-format.md)'s and [0005](0005-config-changeability.md)'s
own lists:

- A config file that is group-writable, world-writable, or owned by a third account refuses the boot,
  naming path and mode; so does one whose directory is group-writable. Both are re-checked on
  `nvs ctl reload`, and a failure there leaves the previous snapshot serving.
- `--config` naming a missing file refuses; `./nvs.toml` missing does not, and the shipped-defaults line
  appears in the boot log. The resolved absolute path is logged in both cases.
- `nvs ctl reload` after the working directory has been replaced re-reads the stored absolute path.
- An `[[include]]` cycle is refused with the chain named; depth past 8 is refused as depth rather than as
  a cycle; an entry with both `path` and `dir`, or neither, is refused.
- `dir` reads only `*.toml` directly inside, in ascending filename order, and a second file added to that
  directory changes the resolved value in exactly the position its name implies.
- The same key in two files resolves to the later one, and `nvs config dump --origin` names both.
  The same key twice in one file still refuses (`E0601`).
- `[capabilities] script.spawn` in a later file replaces the earlier list; a second `[[schedule]]` in an
  included file appends. A capability grant in an included file takes effect.
- An `optional` include that is absent boots; the same entry with a group-writable containing directory
  refuses. An `optional` include that exists but is unreadable, mis-owned or unparseable refuses.
- A relative `cache.dir` in an included file resolves against that file's directory, and the whole tree
  copied to another prefix resolves identically.
- `password_file` yields the file's content with one trailing newline stripped and interior or edge spaces
  preserved; empty, whitespace-only, non-UTF-8 and oversized files refuse; setting both `password` and
  `password_file` refuses; a group-writable secret file refuses and a world-readable one warns.
  `nvs config dump` prints `<secret>` and never the value.
- `nvs serve --mode=development` overrides the file and is itself overridden by a matching `[[app]]`
  block's `mode`; `--set` is not an option the CLI accepts.
- `nvs config check` exits non-zero on each refusal above with no server running, and `nvs ctl config`
  reports the live snapshot including an `optional` include that appeared after boot.
