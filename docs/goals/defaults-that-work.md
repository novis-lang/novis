---
looper: 1
title: "With no `nvs.toml`, the application's own code does ordinary work, and code it did not write gains nothing"
summary: "A grant entry that silently grants nothing is a load error and every refusal prints the line to add; the run kind and the app folder reach every capability decision; a CLI run reads, writes, connects to public hosts and loopback, runs programs and spawns scripts with no grant; server work reads its app folder except dot-files and nvs.toml, writes only its own temporary directories, reaches public hosts, and spawns scripts inside its app folder; net.internal takes CIDR ranges; SQLite :memory: and a file under the file rule need no db.open; Postgres and MySQL reach loopback without TLS; sessions default to a SQLite store in the data folder with no lock between requests; the CSRF door reads a _csrf field from a urlencoded body; Core\\Env::secret returns a secret string and has its feature proofs; a CLI run's own code may raise its memory limit; every example, page and the repository's nvs.toml show the new defaults, every rule this goal changed says what is true, and the checks below pass."
context: [crates/nvs-config/src/capability.rs, crates/nvs-runtime/src/capability.rs, docs/rules/config/no-configuration-file-is-a-complete-configuration.md]
stages:
  - id: 1
    title: a grant entry that grants nothing is an error, and every refusal prints the line to add
    status: done
    context: [crates/nvs-config/src/default.toml, crates/nvs-cli/src/config.rs, crates/nvs-stdlib/src/db/span.rs, docs/rules/security/denial-is-a-runtime-error.md, docs/rules/security/process-exec-capability.md, docs/rules/core-classes/process-is-argv-only.md]
    extra:
      context:
        rules: [security/denial-is-a-runtime-error, config/check-and-dump-audit-the-tree-offline, security/process-exec-capability, core-classes/process-is-argv-only, testing/debug-mode-directive]
  - id: 2
    title: the run kind and the app folder reach every capability decision
    status: done
    context: [crates/nvs-runtime/src/ctx/isolate.rs, crates/nvs-config/src/defaults.rs, crates/nvs-config/src/snapshot.rs, crates/nvs-config/src/mount.rs, docs/rules/security/capability-question-is-grant-and-scope.md]
    extra:
      context:
        rules: [security/capability-question-is-grant-and-scope, config/no-configuration-file-is-a-complete-configuration, config/an-application-is-its-entry-file-path, security/isolate-shares-nothing, expressions/preparation-preserves-behaviour]
  - id: 3
    title: a CLI run reads and writes files freely, and server work reads its app folder and writes only its own temporary directories
    status: done
    context: [docs/rules/security/path-scope-canonicalise-then-prefix.md, docs/rules/core-classes/temporary-dir-sweep.md, tests/hostile/README.md, nvs.toml]
    extra:
      context:
        rules: [config/no-configuration-file-is-a-complete-configuration, security/path-scope-canonicalise-then-prefix, core-classes/temporary-dir-sweep, security/capability-costs-nothing-unasked]
  - id: 4
    title: the application's own code reaches public hosts, and loopback on a CLI run and in development
    status: done
    context: [crates/nvs-config/src/mode.rs, crates/nvs-ext/src/grants.rs, docs/rules/security/net-address-policy.md, docs/rules/config/a-startup-default-is-never-flipped.md]
    extra:
      context:
        rules: [security/net-address-policy, security/outbound-url-is-a-sink, config/a-startup-default-is-never-flipped, config/a-mode-is-five-defaults, security/extension-grants-are-an-intersection]
  - id: 5
    title: SQLite needs only the file rule, and Postgres and MySQL reach loopback without TLS
    status: done
    context: [crates/nvs-stdlib/src/db/open.rs, crates/nvs-db/src/pg.rs, crates/nvs-db/src/mysql.rs, docs/rules/core-classes/db-capabilities.md]
    extra:
      context:
        rules: [core-classes/db-capabilities, core-classes/db-safe-connection-defaults, security/net-address-policy]
  - id: 6
    title: a CLI run starts any program, and a script inside the app folder spawns with no grant
    status: done
    context: [crates/nvs-runtime/src/script.rs, crates/nvs-config/src/schedule.rs, crates/nvs-stdlib/src/process.rs, docs/rules/security/process-exec-capability.md, docs/rules/security/script-spawn-capability.md]
    extra:
      context:
        rules: [security/process-exec-capability, core-classes/process-is-argv-only, core-classes/process-refuses-a-shell-target, core-classes/process-options, security/script-spawn-capability, security/no-runtime-grant]
  - id: 7
    title: with no `[session]` block, sessions live in a SQLite file in the data folder, and requests of one session never wait for each other
    status: done
    context: [crates/nvs-stdlib/src/session.rs, crates/nvs-config/src/session.rs, crates/nvs-config/src/data.rs, docs/rules/http-server/no-session-block-means-the-data-folder-store.md]
    extra:
      context:
        rules: [http-server/no-session-block-means-the-data-folder-store, http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot, http-server/a-session-is-loaded-once-and-written-whole, http-server/session-expiry-belongs-to-the-store]
  - id: 8
    title: the CSRF door reads a `_csrf` field from a urlencoded form
    status: done
    context: [crates/nvs-server/src/route.rs, crates/nvs-cli/src/serve.rs, crates/nvs-stdlib/src/csrf.rs, docs/rules/security/csrf-is-on-by-default.md]
    extra:
      context:
        rules: [security/csrf-is-on-by-default, http-server/request-body-and-upload-total-are-two-caps]
  - id: 9
    title: "`Core\\Env::secret` returns a `secret string`, and has its feature proofs"
    status: done
    context: [crates/nvs-stdlib/src/env.rs, crates/nvs-stdlib/src/http.rs, docs/rules/security/secret-has-no-ambient-source.md, docs/rules/security/tainted-sources.md]
    extra:
      context:
        rules: [security/secret-has-no-ambient-source, security/tainted-sources, core-api/reference-card, testing/feature-proofs]
  - id: 10
    title: a CLI run's own code may raise its memory limit, and the limit error names how
    status: done
    context: [crates/nvs-stdlib/src/config.rs, crates/nvs-config/src/request.rs, crates/nvs-runtime/src/ctx/limits.rs, docs/rules/config/ceilings-are-their-own-directives.md]
    extra:
      context:
        rules: [config/no-configuration-file-is-a-complete-configuration, config/ceilings-are-their-own-directives, config/ini-set-is-core-config-set]
  - id: 11
    title: the repository's `nvs.toml`, the shipped `nvs.toml` and the examples show the new defaults
    status: done
    context: [nvs.toml, crates/nvs-config/src/default.toml]
    extra:
      context:
        rules: [config/no-configuration-file-is-a-complete-configuration, security/an-unmatched-namespace-holds-the-application]
  - id: 12
    title: the website says what the defaults are, and every rule this goal changed says what is true
    status: open
    context: [website/src/content/docs/guides/how-to-use/configuration.mdx, website/src/content/docs/guides/production/security-checklist.mdx]
    extra:
      context:
        rules: [config/no-configuration-file-is-a-complete-configuration, security/package-authority-is-granted-one-line-at-a-time, security/an-unmatched-namespace-holds-the-application]
  - id: 13
    title: the goal's one gate is green
    status: open
    gate: true
    context: [tools/nv/cmd/verify.ts]
checks:
  - id: config-tests
    stage: 1
    name: the configuration crate's tests pass
    run: [bun, nv, verify, --fast, -p, nvs-config]
  - id: runtime-tests
    stage: 6
    name: the runtime's capability and script tests pass
    run: [bun, nv, verify, --fast, -p, nvs-runtime]
  - id: db-tests
    stage: 5
    name: the database drivers' tests pass
    run: [bun, nv, verify, --fast, -p, nvs-db]
  - id: server-tests
    stage: 8
    name: the server's door tests pass
    run: [bun, nv, verify, --fast, -p, nvs-server]
  - id: env-secret-proofs
    stage: 9
    name: Core\Env::secret owes no feature proof
    run: [bun, nv, proofs, --gate, --only, "Core\\Env::secret"]
  - id: owners-closes
    stage: 13
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, defaults-that-work]
    reach: true
    floor: false
---

# With no `nvs.toml`, the application's own code does ordinary work, and code it did not write gains nothing

## Why

An audit ran ordinary tasks on a host with no `nvs.toml` and found almost every capability deny-all,
on a CLI run and on a server alike. Reading a file beside the script, calling a public API, opening
SQLite (`:memory:` included), running `git`, writing a file, spawning a WebSocket script and starting
a session (which needs Redis) all fail. The repository's own `nvs.toml` grants `connect = true` to six
example applications: the habit the strict defaults were meant to prevent is already in the tree.

Three protections already hold with no `nvs.toml`: `tainted` at every sink, checked while compiling;
the outbound address table, which refuses every non-public address even under `connect = true`; and
the per-package grant line goal `namespace-grants` builds. So strictness can follow who wrote the code
and what it can damage instead of applying to everything. Third-party code keeps zero authority.
Request input stays blocked by `tainted` and the address table. The application's own code gets
defaults that work. A written list in `nvs.toml` narrows further for a hardened deployment. The goal
is the same security with less friction for developers and administrators.

## Standing decisions

The twelve numbered decisions are the user's. A detail beyond them is this goal's choice: the startup
row in 3, the read-only open in 4, the per-run temporary directories in 12 and the session file's
name in 8. Each records its reason.

**The terms.** A **CLI run** is `nvs run`, a `#[Command]` method and `nvs test`: the split
`RunKind::is_server_work` already makes for `cpu_time`
(`crates/nvs-runtime/src/ctx/isolate.rs:63`, `rule:config/no-configuration-file-is-a-complete-configuration`).
**Server work** is a request, a connection, a scheduled run, a queued job, and every isolate in their
trees; an isolate takes the kind of the run that spawned it. **The application's own code** is code
under no package's namespace. A package's authority is its grant line intersected with the
application's (`rule:security/grants-are-keyed-on-a-namespace`,
`rule:security/package-authority-is-granted-one-line-at-a-time`), and a default is never part of a
grant line, so a wider application default never widens a package. **The app folder** is the
canonical folder of the entry file the run started from: `Mounted::root`
(`crates/nvs-config/src/mount.rs:67`) for a request or a connection, the `[[schedule]] script`'s
folder for a scheduled run, the worker entry's folder for a queued job, and the entry file's folder
for a CLI run. Where the most specific `[[app]]` block matching that entry names a `root`
(`rule:config/an-application-is-its-entry-file-path`), the app folder is that `root`, so an entry in
`public/` under `[[app]] root = "/srv/www/shop"` reads the whole application. An isolate takes its
tree's app folder.

**One home for the defaults.** A default is a value of the configuration crate's table of defaults
(`crates/nvs-config/src/defaults.rs:370`), as `rule:config/no-configuration-file-is-a-complete-configuration`
requires, and one function in `crates/nvs-config/src/capability.rs` applies it: it takes the snapshot,
the capability, the scope and the run (server work or not, and the app folder) and returns the grant.
Every asker goes through it: the runtime's `granted` (`crates/nvs-runtime/src/capability.rs:242`),
the schedule check at boot (`crates/nvs-config/src/schedule.rs:247`), an extension's caller grants
(`crates/nvs-ext/src/grants.rs:69`) and `Core\Cap::has`. The host that builds a context stores the app
folder on it beside its `RunKind`, so the procedure stays pure
(`rule:security/capability-question-is-grant-and-scope`). **A written key replaces its default
whole**, per key and per run kind: `read = ["data/"]` narrows reading and leaves writing at its
default, and `[]` is a written key that grants nothing. A block header with no key under it says
nothing, as today. A spawn's `grants:` list still only subtracts (`rule:security/isolate-shares-nothing`).

**1. Files.** A CLI run has no `fs.read` and no `fs.write` limit unless one is written. Server work
reads the app folder and nothing else. The default read excludes every path with a component that
starts with `.` below the app folder (`.env`, `.git`) and every file named `nvs.toml`, checked on the
canonical path (`rule:security/path-scope-canonicalise-then-prefix`). Any write, and any read outside
the app folder, needs a written line. A written `fs.read` list replaces the default.

**2. Outbound hosts (`net.connect`).** The application's own code may reach any public host by default,
on a CLI run and in server work. The address table (`rule:security/net-address-policy`,
`crates/nvs-config/src/capability.rs:262`) still refuses every non-public address. A written
`connect = [...]` list narrows to an allowlist. A `tainted` URL stays a compile error
(`rule:security/outbound-url-is-a-sink`). `connect_to` and `downgrade` keep no default.

**3. Private and loopback addresses (`net.internal`).** An entry may be a CIDR range (`172.18.0.0/16`)
beside an exact address; a hostname and `true` stay refused. Loopback (`127.0.0.0/8`, `::1`) is
allowed by default on a CLI run and in `development` mode. A production server still needs a written
line for any private or loopback address. The user asked for this as a row of
`rule:config/a-mode-is-five-defaults`, and that rule requires every row be a value a request could set
for itself, which a capability is not. So the row is a **startup row** of
`rule:config/a-startup-default-is-never-flipped` (`Value::Startup`, `crates/nvs-config/src/defaults.rs:291`):
chosen from the mode the configuration names, never re-derived by a runtime mode flip. A program that
flips to `development` gains no loopback. `rule:config/a-mode-is-five-defaults`' list of what a mode
does not govern gains one line that points at the startup row.

**4. SQLite.** `:memory:` and the empty path need no grant. A SQLite file needs only what the file rule
gives (decision 1): free on a CLI run. In server work the app folder is readable, so a file there
opens read-only (SQLite's read-only open flag) and a write throws SQLite's read-only error; a database
the server writes needs a written `fs.write` root. A `file:` URI keeps today's check. `db.open` is asked
only for a network database. Today `Db::open` asks `db.open`, `fs.read` and `fs.write` in turn
(`crates/nvs-stdlib/src/db/open.rs:978`), and a relative `db.open` entry matches nothing
(`rule:core-classes/db-capabilities`).

**5. Database TLS.** Postgres and MySQL (MariaDB included) connect without TLS to a loopback address
(`127.0.0.0/8`, `::1`) only, because that traffic never leaves the machine. Every other address keeps
verified TLS, with no setting to turn it off. The loopback address must still pass the address table
and `net.internal` (decision 3), or come from an operator's `[db.<name>]` block. Today
`crates/nvs-db/src/pg.rs:1127` and `crates/nvs-db/src/mysql.rs:273` refuse a server with no TLS. SQL
Server is unchanged. Rules `rule:core-classes/db-capabilities` and
`rule:core-classes/db-safe-connection-defaults`.

**6. Running programs (`process.exec`).** A CLI run may run any program, and a bare name such as `git`
is looked up on `PATH`. Server work needs a grant, which may name a program (`exec = ["git"]`, resolved
through `PATH` once at boot to an absolute path, and refused at boot when nothing is found) or a path.
There is still no shell; a `.bat`, `.cmd` or `.ps1` target stays refused
(`rule:core-classes/process-refuses-a-shell-target`); the child keeps its base environment
(`rule:core-classes/process-options`). Code: `crates/nvs-runtime/src/capability.rs:1492`, and the
relative-path refusal at `:128`.

**7. Spawning scripts (`script.spawn`).** A script inside the app folder spawns with no grant, on a CLI
run and in server work: `require` already trusts that code. A script outside it needs a `spawn` root,
and a child still only narrows (`rule:security/no-runtime-grant`). The default read's dot-file and
`nvs.toml` exclusions apply to it too. Code: `crates/nvs-runtime/src/script.rs:477`,
`crates/nvs-config/src/schedule.rs:247`. Rule `rule:security/script-spawn-capability`.

**8. Sessions.** The `db` session store is finished, and with no `[session]` block the store is a
SQLite file in Novis's own data folder (`<data>/session.sqlite`, `crates/nvs-config/src/data.rs:459`),
in WAL mode. Every server process on the machine that shares the data folder reads it, so the reason
the memory store is refused (`rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`)
still holds; `shared` stays the store for several machines. With no usable data folder, `start` throws
naming `[session]`, as today. There is still no session lock
(`rule:http-server/a-session-is-loaded-once-and-written-whole`): the user requires that parallel
requests of one session never wait for each other, which is the PHP session-lock problem. A request
reads its row at `start` and writes it whole at the end in one short statement. Two concurrent
requests of one session both complete, and the last write wins. Expired rows are deleted by a sweep
(`rule:http-server/session-expiry-belongs-to-the-store`). This changes
`rule:http-server/no-session-block-means-the-data-folder-store`.

**9. The CSRF form field.** With a `[http] csrf_key`, the door also reads a `_csrf` field from an
`application/x-www-form-urlencoded` body. `[limits] request_body` already caps that body. A multipart
body carries its token in the `x-csrf-token` header or in a `_csrf` query parameter, which the door
also reads; a large upload is never buffered before the check. `TOKEN_HEADER`'s doc comment
(`crates/nvs-server/src/route.rs:162`) gives the reason the body is not read; that reason is narrowed
to multipart, not dropped. The `Core\Csrf::issue` reference text says "Put it in the form"
(`crates/nvs-stdlib/src/csrf.rs:170`) and is rewritten to name the field. Rule
`rule:security/csrf-is-on-by-default`.

**10. `Core\Env::secret('NAME')`** returns `?secret string`, not `tainted`. A `secret` is accepted where
a credential belongs (an outbound header, a password field) and never printed or logged. `Core\Env::get`
stays `?tainted string`, because an environment value can come from whoever started the process.
Today an outbound header refuses `tainted` (`crates/nvs-stdlib/src/http.rs:2241`), so every API key
read from the environment needs `Core\Taint::assertTrusted`. The name is a sink, as `get`'s is. It is
the second member whose contract is confidentiality, beside the terminal password prompt
(`rule:security/secret-has-no-ambient-source`). It is a new `Core` member and owes feature proofs
(`rule:testing/feature-proofs`).

**11. CLI memory.** A CLI run keeps the 128 MiB default. On a CLI run with no `[limits.hard] memory`
written, the application's own code may raise its limit with `Core\Config::set`. Server work is
unchanged: a raise goes only up to `[limits.hard]`. `Core\Config::set` does not distinguish package code
today (`crates/nvs-stdlib/src/config.rs:209`), and no code is package code until goal `namespace-grants`
lands; stage 10 records that a package may only lower its limit, and the check is a checkpoint of that
goal's stage 3, which this stage builds instead if that goal's gate already holds. The memory-limit error says "the request" on every run (`crates/nvs-runtime/src/ctx/limits.rs:450`);
it names the program on a CLI run, and names `[limits] memory` and the `Core\Config::set` call. Rules
`rule:config/no-configuration-file-is-a-complete-configuration`,
`rule:config/ceilings-are-their-own-directives`.

**12. Server writes.** With no configuration, server work writes only the directories
`Core\IO::temporaryDir` created for this run, which the context already tracks for the sweep
(`crates/nvs-runtime/src/capability.rs:1273`). That is narrower than the whole temporary root, so one
request cannot read or write another's directory. Every other write needs a written line, so nothing a
server writes lands next to code it would run. Rule `rule:core-classes/temporary-dir-sweep`.

**Bugs, fixed in stage 1 whatever the defaults.**

- `crates/nvs-config/src/default.toml:158` shows `connect = ["api.example.com:443"]` and `:221`
  `open = ["db.example.com:5432"]`, repeated at `:874`, `:902`, `:1048` and `:1076`. A grant compares the
  bare host, so both match nothing: the release binary refuses the `:443` form and accepts the bare
  host. An entry with a port is refused at load, with a message naming the bare host, and every example
  is fixed.
- `["*"]`, `true` on the seven grants with no `true` spelling (`crates/nvs-config/src/capability.rs:490`),
  and a relative `db.open` entry each grant nothing silently. `nvs config check` and the boot report
  each as an error naming the fix.
- Every refusal (`crates/nvs-runtime/src/capability.rs:271`) prints the exact line to add under its
  table, such as `read = ["data/"]` under `[capabilities.fs]`: the narrowest root for the refused path,
  or the refused host. The address-table refusal (`:546`) gains the same help line naming
  `[capabilities.net] internal`.
- `rule:core-classes/process-is-argv-only` says the argv elements refuse `tainted`, and
  `rule:security/process-exec-capability` says they are data; the code's argv row is `Qual::Neutral`
  (`crates/nvs-stdlib/src/process.rs:170`). The latter also says a bare name resolves against the
  current directory (`docs/rules/security/process-exec-capability.md:16`), and the code refuses it. A
  conformance case settles the argv question against the code, and both rules say what it shows and what
  decision 6 makes true.
- `debug.trace` without its grant traces nothing and says nothing
  (`crates/nvs-stdlib/src/db/span.rs:57`). It reports once per run.

**Kept strict, recorded as declined relaxations** in the rule each belongs to: `net.listen`, the four
`tls.*` relaxations, `ldap.cleartext`, `net.downgrade`, `db.schema`, `queue.purge`, `cache.shared`,
`mail.send`, every `debug.*` grant, the named `[db.*]` and `[ldap.*]` blocks, CORS closed by default,
and the cookie and security-header defaults. Each one either turns a check off or reaches something an
operator has to name. Also declined: loopback always allowed on a production server (it gives up SSRF
protection for every service on localhost); `Env::get` untainted; a writable `storage/` by default (a
written upload could land beside code a default spawn would run); and no CLI memory limit at all.

**Hand-vendored code** with no grant line holds the application's authority
(`rule:security/an-unmatched-namespace-holds-the-application`), which is now wider. The audit warning
that rule names is the mitigation, and goal `namespace-grants` must keep it. Stage 12 says so in that
rule and in `rule:security/package-authority-is-granted-one-line-at-a-time`: a package is checked
against its own line alone, never against a default.

**The attacks and the conformance cases keep their refusals.** An attack runs as a CLI run under the
repository's `nvs.toml`, so a default that opens a CLI run would turn an attack's refusal into a pass.
The first stage that switches a default on adds one `[[app]] root = "tests/hostile"` block to
`nvs.toml` that writes that key as `[]`, and each later stage adds its own key there. An attack that
tests a default itself writes its own `[[app]] entry` block. A `.nvst` case or a Rust test that relied
on deny-all writes the empty key in its own configuration. Each stage that switches a default keeps
the tree green this way; no stage loosens an attack.

**Trade-offs.** Security: the application's own code gains authority it was already able to obtain with
one line. Third-party code gains nothing, and request input is still stopped by `tainted` and the
address table. A production server keeps loopback and private addresses closed, writes only its own
temporary directories, and runs no program without a grant. The default read reaches the app folder
but no dot-file and no `nvs.toml`. The cost is that a hardened deployment now narrows by writing lists.
Latency: the default is one more branch on the capability check, which is never on a hot path
(`rule:security/capability-costs-nothing-unasked`); the dot-file check is one pass over the canonical
path's components below the app folder. The CSRF form read buffers a urlencoded body before the
handler, which the handler's own form parse would have buffered anyway. Simplicity: a first program
needs no `nvs.toml`, and the ordinary tasks above work. The defaults are one function and one table,
not a rule per door. Memory: one path per context for the app folder; the CSRF read holds up to
`[limits] request_body` per checked urlencoded request in flight, bounded and attributed to that
request; the session store keeps nothing in memory between requests.

**Lanes.** Stages 1 and 2 come first, in order: both touch `crates/nvs-config/src/capability.rs` and
`crates/nvs-runtime/src/capability.rs`. Stages 3, 4 and 6 share those files and the hostile block in
`nvs.toml`, and run after 2 in that order. Stage 5 runs after 3 (it uses the file default) and touches
`db/open.rs`, `pg.rs` and `mysql.rs`, disjoint from 4 and 6. Stages 7 (`session.rs`), 8 (`route.rs`,
`serve.rs`, `csrf.rs`), 9 (`env.rs`, `http.rs`) and 10 (`config.rs`, `request.rs`, `limits.rs`) need
nothing from stages 1 to 6, touch disjoint files, and may run as side goals in any order. Stage 11 runs
after 1 to 10, then stage 12, and stage 13 last. `crates/nvs-cli/src/runner.rs` is touched only by
stage 8.

**Stage 13 is the gate stage**, as every goal's last stage is (`AGENTS.md` § *Session workflow*).
Where a changed default breaks an existing test or case, the slice writes the narrowing the test
relied on, as stages 3 and 4 did, and runs `bun nv rules --check` when it changed a rule.

**Gates on other goals.** None to start. Stage 10's package check waits on goal `namespace-grants`:
`crates/nvs-config/src/grants.rs` exists. Until it does, stage 10 records the duty in the rule, and
the check stays a checkpoint of goal `namespace-grants`' stage 3.

## Stages

### 1. a grant entry that grants nothing is an error, and every refusal prints the line to add

**Does:** the bugs above are fixed, with no default changed.

- [x] A `host:port` entry in `net.connect`, `db.open` and every other host list is refused at load with a message naming the bare host, and the six examples in the shipped file drop the port (`rule:config/check-and-dump-audit-the-tree-offline`). `crates/nvs-config/src/capability.rs:921`, `crates/nvs-config/src/default.toml:158`
  - [x] `capability::validate` runs in `resolve` after `anchor`, over `[capabilities]`, every `[[app]]` and every `[[schedule]] grants`
  - [x] a port, a scheme, a user or a path on a host-list entry is refused, and the help names the bare host
  - [x] the six shipped examples drop the port, and the comment above each says an entry is a host alone
- [x] `["*"]`, `true` on a grant with no `true` spelling and a relative `db.open` entry are each an error from `nvs config check` and at boot, naming the fix (`rule:config/check-and-dump-audit-the-tree-offline`). `crates/nvs-config/src/capability.rs:490`, `crates/nvs-cli/src/config.rs:585`
  - [x] `*`, `*.` and a path-grant `*` refused; `*.x` refused outside `db.open` and `ldap.open`
  - [x] `true` refused on the seven grants with no `true` spelling, and on `net.internal`
  - [x] a relative `db.open` path refused with its absolute form as the fix; a non-endpoint `net.listen` entry and a non-address `net.internal` entry refused too
  - [x] the rule says what is refused
- [x] Every refusal prints the line to add and its table, and the address-table refusal names `[capabilities.net] internal` (`rule:security/denial-is-a-runtime-error`). `crates/nvs-runtime/src/capability.rs:271`, `crates/nvs-runtime/src/capability.rs:546`, `tests/conformance/cap/a-denial-names-the-file-and-the-table-a-grant-is-written-in.nvst:1`
  - [x] `nvs_config::capability::grant_line` and `internal_line` build the line; `denial()` and the address refusal print it
  - [x] every conformance case, example output and reference page that pinned the old text says the new one
  - [x] the rule says what the help line is
- [x] A conformance case passes a `tainted` argv element, and `rule:core-classes/process-is-argv-only` and `rule:security/process-exec-capability` both say what it shows and drop the current-directory sentence. `docs/rules/security/process-exec-capability.md:16`, `docs/rules/core-classes/process-is-argv-only.md:6`, `crates/nvs-stdlib/src/process.rs:170`
  - [x] `process-an-argv-element-that-is-tainted-does-not-compile.nvst` pins `E0401`
  - [x] both rules say a tainted argument does not compile, and a bare name is refused
- [x] `debug.trace` without its grant writes one warning per run (`rule:testing/debug-mode-directive`). `crates/nvs-stdlib/src/db/span.rs:57`
  - [x] `QueryWatch` notes the missing grant and `file` writes one warning per process, naming `trace = true`
  - [x] the rule says so
- [x] Tests: each refused spelling, the bare-host message, and a refusal's help line for a path, a host and an address. `crates/nvs-config/tests/capability.rs:1174`
  - [x] each refused spelling and its help line (`a_grant_entry_that_allows_nothing_is_refused_naming_the_line_to_write`), the accepted spellings beside them, and the resolve wiring (`tests/resolve.rs`)
  - [x] a refusal's help line for a path, a host and an address (`the_line_a_refusal_prints_loads_and_grants_what_was_refused`)

### 2. the run kind and the app folder reach every capability decision

**Does:** the one function that applies a default exists and every asker goes through it, with every
default still deny-all, so nothing observable changes.

**Split:** first, an `Explore` subagent lists every caller of `Capabilities::allows`, `allows_host`,
`allows_name`, `allows_resolved`, `allows_unscoped` and `holds`, as `file:line` and one line each.

- [x] The app folder as the standing decision defines it, computed by one function from the entry path and the matching `[[app]]` blocks, and stored on the context beside its `RunKind` by every host that builds one (`rule:config/an-application-is-its-entry-file-path`). `crates/nvs-config/src/snapshot.rs:520`, `crates/nvs-config/src/mount.rs:67`, `crates/nvs-runtime/src/ctx/isolate.rs:63`
- [x] The defaults function: snapshot, capability, scope and run in, grant out, with a written key replacing its default whole (`rule:security/capability-question-is-grant-and-scope`). `crates/nvs-config/src/capability.rs:694`, `crates/nvs-config/src/defaults.rs:370`
- [x] The runtime, the schedule check, an extension's caller grants and `Core\Cap::has` ask through it, so a check made while compiling and one made at run time agree (`rule:expressions/preparation-preserves-behaviour`). `crates/nvs-runtime/src/capability.rs:242`, `crates/nvs-config/src/schedule.rs:247`, `crates/nvs-ext/src/grants.rs:69`
- [x] An isolate takes its tree's app folder and run kind, and a `grants:` list still only subtracts (`rule:security/isolate-shares-nothing`). `crates/nvs-runtime/src/ctx/isolate.rs:63`
- [x] Tests: the app folder of a mount, a schedule, a CLI entry and an entry under an `[[app]] root`; an isolate inherits it. `crates/nvs-config/tests/capability.rs:1`

### 3. a CLI run reads and writes files freely, and server work reads its app folder and writes only its own temporary directories

**Does:** decisions 1 and 12 are on, and every attack and case that relied on deny-all writes its
narrowing.

**Split:** first, an `Explore` subagent lists every attack, `.nvst` case and Rust test that expects an
`fs.read` or `fs.write` refusal with no grant written. After the defaults land, one writing subagent
adds the narrowing lines.

- [x] A CLI run's `fs.read` and `fs.write` default to everything, and server work's `fs.read` to the app folder, without dot-file components below it and without `nvs.toml` (`rule:config/no-configuration-file-is-a-complete-configuration`, `rule:security/path-scope-canonicalise-then-prefix`). `crates/nvs-config/src/capability.rs:694`, `crates/nvs-config/src/defaults.rs:370`
- [x] Server work writes and reads the directories `Core\IO::temporaryDir` made for this run with no grant (`rule:core-classes/temporary-dir-sweep`). `crates/nvs-runtime/src/capability.rs:1273`
- [x] The boot log's shipped-defaults line names the app folder a server reads. `crates/nvs-cli/src/serve.rs:220`
- [x] The `[[app]] root = "tests/hostile"` block writes `read = []` and `write = []`, and each listed case writes its own narrowing. `nvs.toml:19`
- [x] Tests: a server reads a file in its app folder, and is refused `.env`, `.git/config`, `nvs.toml`, a symlink out of the folder, a sibling folder and any write; a CLI run reads and writes outside its folder; one request cannot reach another's temporary directory. `crates/nvs-runtime/src/capability.rs:2271`

### 4. the application's own code reaches public hosts, and loopback on a CLI run and in development

**Does:** decisions 2 and 3 are on.

- [x] `net.connect` defaults to every host, with the address table asked of every resolved address as before (`rule:security/net-address-policy`). `crates/nvs-config/src/capability.rs:615`, `crates/nvs-runtime/src/capability.rs:546`
- [x] A `net.internal` entry may be a CIDR range, and a hostname, `true` and a malformed range are refused at load (`rule:security/net-address-policy`). `crates/nvs-config/src/capability.rs:856`
- [x] Loopback is a startup row: allowed on a CLI run and when the configuration names `development`, never moved by a runtime flip; `rule:config/a-mode-is-five-defaults` points at it (`rule:config/a-startup-default-is-never-flipped`). `crates/nvs-config/src/defaults.rs:291`, `crates/nvs-config/src/mode.rs:69`
- [x] An extension's hosts are still the intersection with its `[[extension]] grants` and its manifest (`rule:security/extension-grants-are-an-intersection`). `crates/nvs-ext/src/grants.rs:69`
- [x] The hostile block writes `connect = []`, and each listed case writes its own narrowing. `nvs.toml:19`
- [x] Tests: a public host with no grant, a private address still refused, a CIDR entry, loopback on a CLI run and in development, loopback refused in production and after a flip to development. `crates/nvs-config/tests/capability.rs:1`

### 5. SQLite needs only the file rule, and Postgres and MySQL reach loopback without TLS

**Does:** decisions 4 and 5 are on.

- [x] `:memory:` and the empty path need no grant, a file asks `fs.read` and `fs.write` alone, and a file only readable opens read-only (`rule:core-classes/db-capabilities`). `crates/nvs-stdlib/src/db/open.rs:978`
- [x] Postgres accepts a server with no TLS on a loopback address only (`rule:core-classes/db-safe-connection-defaults`). `crates/nvs-db/src/pg.rs:1127`
- [x] MySQL and MariaDB do the same. `crates/nvs-db/src/mysql.rs:273`, `crates/nvs-db/src/mysql.rs:988`
- [x] Tests: `:memory:` on a server with no grant, a file in the app folder read and refused a write, a file on a CLI run, and a no-TLS server accepted on `127.0.0.1` and refused on any other address. `crates/nvs-db/src/pg.rs:1127`, `crates/nvs-stdlib/src/db/open.rs:2014`

### 6. a CLI run starts any program, and a script inside the app folder spawns with no grant

**Does:** decisions 6 and 7 are on.

- [x] A CLI run's `process.exec` defaults to every program, and a bare name is looked up on `PATH` on a CLI run only (`rule:security/process-exec-capability`). `crates/nvs-runtime/src/capability.rs:1492`, `crates/nvs-runtime/src/capability.rs:128`
- [x] A server grant entry that is a bare name resolves through `PATH` once at boot, and a name nothing answers stops the boot (`rule:security/process-exec-capability`). `crates/nvs-config/src/capability.rs:921`
- [x] A script inside the app folder spawns with no grant, from a door and from `[[schedule]]` alike (`rule:security/script-spawn-capability`). `crates/nvs-runtime/src/script.rs:477`, `crates/nvs-config/src/schedule.rs:247`
- [x] The hostile block writes `exec = []` and `spawn = []`, and each listed case writes its own narrowing. `nvs.toml:19`
- [x] Tests: `git --version` by name on a CLI run, a bare name refused in server work with no grant and started with one, a `.cmd` still refused, a script in the app folder spawned and one outside refused. `crates/nvs-runtime/src/capability.rs:2271`

### 7. with no `[session]` block, sessions live in a SQLite file in the data folder, and requests of one session never wait for each other

**Does:** decision 8 is on.

- [x] The `db` store reads a row at `start` and writes it whole at the end, with no lock (`rule:http-server/a-session-is-loaded-once-and-written-whole`). `crates/nvs-stdlib/src/session/sqlite.rs:138`
- [x] No `[session]` block selects a SQLite file in the data folder, in WAL mode, and no usable data folder throws naming `[session]` (`rule:http-server/no-session-block-means-the-data-folder-store`). `crates/nvs-stdlib/src/session.rs:617`, `crates/nvs-config/src/data.rs:459`
- [x] Expired rows are deleted by a sweep (`rule:http-server/session-expiry-belongs-to-the-store`). `crates/nvs-stdlib/src/session/sqlite.rs:203`
- [x] Tests: two concurrent requests of one session both complete without waiting on each other and the last write wins; a session survives a second server process on the same data folder; an expired row is swept. `tests/conformance/core/session-start-with-no-block-opens-the-data-folder-store.nvst:1`, `crates/nvs-stdlib/src/session/sqlite.rs:256`

### 8. the CSRF door reads a `_csrf` field from a urlencoded form

**Does:** decision 9 is on.

- [x] With a key, a checked urlencoded request with no header token is read up to `[limits] request_body` and its `_csrf` field checked; a multipart body is never read (`rule:security/csrf-is-on-by-default`). `crates/nvs-server/src/route.rs:211`, `crates/nvs-cli/src/serve.rs:1187`
- [x] A `_csrf` query parameter is read for every checked request, and `TOKEN_HEADER`'s doc comment says why the multipart body stays unread. `crates/nvs-server/src/route.rs:162`
- [x] The in-process door the runner uses agrees. `crates/nvs-cli/src/runner.rs:912`
- [x] `Core\Csrf::issue`'s reference text names the `_csrf` field and the header. `crates/nvs-stdlib/src/csrf.rs:170`
- [x] Tests: a form field passes, a wrong one is refused, a body past the limit is refused before the handler runs, and a multipart upload passes with the header and with the query parameter. `crates/nvs-server/src/route.rs:523`

### 9. `Core\Env::secret` returns a `secret string`, and has its feature proofs

**Does:** decision 10 is on, and the member owes no feature proof.

**Split:** after the member lands, one writing subagent writes the about page, the three examples and
the bench, then a second writes the attack, each held to § *Text an end user reads* in `AGENTS.md`.

- [x] The five edits for `secret`: the row with its name a sink, the card, the body, the `address()` arm and three `.nvst` cases (`rule:core-api/reference-card`). `crates/nvs-stdlib/src/env.rs:124`, `crates/nvs-stdlib/src/env.rs:453`, `tests/conformance/core/env-secret-is-accepted-by-an-outbound-header-and-never-printed.nvst:1`
- [x] `rule:security/secret-has-no-ambient-source` names the second originating member, and `rule:security/tainted-sources` says why its value is not `tainted`. `docs/rules/security/secret-has-no-ambient-source.md:1`, `docs/rules/security/tainted-sources.md:1`
- [x] The about page and three examples: an API key in an outbound header, a database password, and a missing variable. `docs/examples/core/Env/secret/about.md:1`
- [x] The bench. `benches/members/core/Env/secret.nvs:1`
- [x] The attack: a program tries to print, log and return the value in a response. `tests/hostile/core/Env/secret/01-print-a-secret-from-the-environment.nvs:1`
- [x] `bun nv proofs --id 'Core\Env::secret'` owes nothing (`rule:testing/feature-proofs`). `docs/examples/core/Env/secret/about.md:1`

### 10. a CLI run's own code may raise its memory limit, and the limit error names how

**Does:** decision 11 is on.

- [x] On a CLI run with no `[limits.hard] memory` written, `Core\Config::set('memory', ...)` raises the limit; server work is unchanged (`rule:config/ceilings-are-their-own-directives`). `crates/nvs-config/src/request.rs:359`, `crates/nvs-stdlib/src/config.rs:209`
- [x] A package may only lower its limit: recorded in `rule:security/package-authority-is-granted-one-line-at-a-time`. The check is goal `namespace-grants`' stage 3 checkpoint, unless `crates/nvs-config/src/grants.rs` already exists, in which case this stage adds it and ticks that checkpoint. `crates/nvs-stdlib/src/config.rs:209`
- [x] The memory-limit error names the program on a CLI run, `[limits] memory` and the `Core\Config::set` call (`rule:config/no-configuration-file-is-a-complete-configuration`). `crates/nvs-runtime/src/ctx/limits.rs:450`
- [x] Tests: a CLI raise past 128M, a server raise refused, and a written hard ceiling bounding a CLI raise. `crates/nvs-stdlib/src/config.rs:530`

### 11. the repository's `nvs.toml`, the shipped `nvs.toml` and the examples show the new defaults

**Does:** every grant the new defaults make unnecessary is gone from the repository's `nvs.toml`, and
the shipped file's comments and the examples state the defaults.

**Split:** first, an `Explore` subagent lists every `[app.capabilities]` line in `nvs.toml` that a
default now covers, every example and `about.md` under `docs/examples/` that states deny-all or writes
such a grant, and every conformance case that does. Then one writing subagent edits `nvs.toml`, and a
second the examples, held to § *Text an end user reads* in `AGENTS.md`.

- [x] The six `connect = true` blocks and every other covered grant are removed, and the hostile block stays (`rule:config/no-configuration-file-is-a-complete-configuration`). `nvs.toml:137`
- [x] The shipped file's `[capabilities]` comments state each default by run kind. `crates/nvs-config/src/default.toml:141`
- [x] Every listed example and `about.md` says what is true. `docs/examples/core/Env/get/about.md:1`
- [x] Every program in the tree still runs under the trimmed file. `nvs.toml:19`

### 12. the website says what the defaults are, and every rule this goal changed says what is true

**Does:** the pages are true, every rule this goal changed says what is true, and the goal owes nothing.

**Split:** first, an `Explore` subagent lists every page under `website/src/content/docs/` that states a
capability default. Then one writing subagent edits them, held to § *Text an end user reads* in
`AGENTS.md`.

- [ ] The configuration page and the security checklist say what each run kind may do with no `nvs.toml`, and how to narrow it. `website/src/content/docs/guides/how-to-use/configuration.mdx:1`, `website/src/content/docs/guides/production/security-checklist.mdx:1`
- [ ] Every other listed page. `website/src/content/docs/guides/cookbook/upload-file.mdx:1`
- [ ] `rule:security/an-unmatched-namespace-holds-the-application` and `rule:security/package-authority-is-granted-one-line-at-a-time` say a default never reaches a package and the audit warning is the mitigation for hand-vendored code. `docs/rules/security/an-unmatched-namespace-holds-the-application.md:1`, `docs/rules/security/package-authority-is-granted-one-line-at-a-time.md:1`
- [ ] Each rule this goal changed says what is true and lists its declined relaxations, and `bun nv rules --check` passes: `config/no-configuration-file-is-a-complete-configuration`, `security/capability-question-is-grant-and-scope`, `security/net-address-policy`, `config/a-startup-default-is-never-flipped`, `core-classes/db-capabilities`, `security/script-spawn-capability`, `http-server/no-session-block-means-the-data-folder-store`, `config/ceilings-are-their-own-directives`. `docs/rules/config/no-configuration-file-is-a-complete-configuration.md:1`

### 13. the goal's one gate is green

**Does:** stages 5 to 12 checked with a build alone, so this stage verifies
the whole goal once and fixes every failure in one pass.

**Split:** the session starts `bun nv affected --run` with `run_in_background` and ends its turn to
wait. It groups the failures by cause. A group whose fix touches files no other group touches may go
to a writing subagent, one after another. Then the gate runs once more.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass. A test whose expectation a new default changed writes the narrowing it relied on, and no test is deleted, renamed or loosened to pass. `docs/goals/defaults-that-work.md:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`
- [ ] `bun nv verify --doc` names no broken link, and `bun nv owners --closes defaults-that-work` passes. `docs/goals/defaults-that-work.md:1`

## Handoff

### State

Stages 1 to 11 are done. Stage 4 is committed without a green gate: its gate was stopped after a
test hung. Stage 12 checks with a build alone, and stage 13, the gate stage, verifies the goal.

**The last sweep's reds were already fixed.** The `env-secret-proofs` log is from 22:32, before
`target/release/nvs.exe` was rebuilt at 22:40; the check is green at HEAD. The nightly run
`39567373487` is the same `afbb1edc2` run the earlier handoff covered.

**Stage 11 (the files show the defaults), `6d9340431` and `c775ff1cc`:**

- `nvs.toml` lost every grant a CLI default now gives: the `examples/{files,tempdir,upload,process,http}.nvs`
  blocks, the Csv, Serialize, Http-Stream, IO, IO-File, IO-Metadata, Db/open and Db/inList docs
  roots and benches, `connect = true` on the allowUrl, Http-Client and Http-Part docs, and the
  loopback `internal` lines of net-echo, Ldap/open and Ldap-Tls. Kept: every `tests/hostile`
  grant (the `[]` block closes the defaults there), `spawn = true` on `root = "."`
  (`config/app/01` prints it, `config/capabilities` takes it back), `config/capabilities`'
  `read = true` (`Core\Config::get` renders only a written key), and the narrowings an example
  prints. Each comment says what is true.
- The shipped file's `[capabilities]` banner defines a CLI run, server work and the app folder.
  `spawn`, `read`, `write`, `connect`, `internal` and `exec` end in `# default: <what unset gives>`,
  as do their `[schedule.grants.*]` and `[app.capabilities.*]` copies, which
  `bun nv directives --check-template` requires. `schedule.limits.memory` and
  `app.limits.hard.memory` now match `limits.hard.memory`'s trailer. `crates/nvs-config/tests/defaults.rs`
  and `directives.rs` pass.
- Examples: about pages for IO, Http, Http-Client, Process, Db/open, Storage, Zip, the
  spreadsheet features and the memory pages say what each run kind may do. 25 per-example
  `nvs.toml`s are deleted (20 of them landed in `6d9340431`, staged by the examples agent), and the
  Net ones keep only `listen`/`local`. `Process/{run,spawn}` keep `exec = ["/bin", "/usr/bin"]`,
  and `Net/connect` and `Net-Datagram/send` keep `connect = ["127.0.0.1"]`, for the refusal their
  `02` prints. `Process-Result/exitCode/03` now tests for the root `nvs.toml`.
- Runs: the programs behind every changed block and page (about 150 features, examples and
  attacks) match their `.out` on the debug binary. Not run: the whole roster (the guard leaves that
  to the gate), and the `requires: unix` Process-Handle and Process-Result examples whose
  `nvs.toml` was deleted.
- **Expected red at stage 13:** an example or attack outside the features above whose output
  relied on a removed grant, and a Process-Handle or Process-Result example on Linux.

**Stage 10 (CLI memory), `1d56487ff`:**

- `limits.hard.memory`'s row in `crates/nvs-config/src/defaults.rs` is the new
  `Value::InheritServing("limits.memory")`: the `[limits]` value in server work, and nothing on a
  CLI run. So `Request::ceiling` (`request.rs`) finds no bound and a CLI `set` checks only the
  unit. `Core\Config::get('limits.hard.memory')` is `null` on a CLI run.
- `Ctx::memory_breach` (`crates/nvs-runtime/src/ctx/limits.rs`) says "the program" or "the
  request" (`memory_run`), and adds a line from `memory_help`: ``help: raise `memory` under
  `[limits]` in nvs.toml, or call `Core\Config::set('memory', '<2x ceiling>M')` in the program``,
  plus ``, up to `[limits.hard] memory` `` where a ceiling bounds the call.
- The repository's `nvs.toml` writes `[limits.hard] memory = "512M"`, so attacks and examples keep
  their bound, and the hostile block needs no new key.
- Tests, each passed once: `a_cli_run_raises_its_memory_freely_until_a_hard_ceiling_is_written`
  and `the_memory_limit_error_names_the_run_and_how_to_raise_it` (`crates/nvs-stdlib/src/config.rs`);
  `an_unwritten_hard_ceiling_is_the_value_the_run_started_with` now runs as a request. Two tests in
  `crates/nvs-config/tests/request.rs` were updated, and that file passes. Ten memory-ceiling
  `.nvst` cases expect the new error with its help line, and
  `config-unconfigured-host-answers-as-an-empty-configuration.nvst` shows `raise=ok`. All eleven
  pass, as do the other eight cases that call `Core\Config::set('memory', ...)`.
- Rules: `config/ceilings-are-their-own-directives` (the exception, the error text, two Declined
  lines), `config/no-configuration-file-is-a-complete-configuration`, and
  `security/package-authority-is-granted-one-line-at-a-time` (a package may only lower its limits;
  the check stays `namespace-grants`' stage 3 checkpoint, since `crates/nvs-config/src/grants.rs`
  does not exist). The shipped file's `[limits.hard]` comment says the same.
- **Expected red at stage 13:** a Rust test or `.nvst` case outside those above that expects
  `the request exceeded its memory limit` from a CLI run, or a CLI `set('memory', ...)` above the
  start refused with no `[limits.hard]` written. `crates/nvs-host/src/group.rs:1343` uses the text
  as an opaque string and is not affected.

**Stage 9 (`Core\Env::secret`), `a77e38609` and the next commits:**

- The member, its card, the rules and three `.nvst` cases landed in `a77e38609`. The WIP commit
  `54bbc5fd0` (the about page, three examples, the bench and the Rust test
  `secret_reads_the_value_get_reads_and_null_for_a_name_nothing_set`) is kept as it was; the test
  passed once, and the three examples run green.
- The attack is `tests/hostile/core/Env/secret/01-print-a-secret-from-the-environment.nvs`
  (`expect-refusal`): `echo`, `print`, a log record, `Core\Response::text`, an interpolated string
  and a `tainted` name. Each step is refused at compile time.
- `env-secret-proofs` was red only because `target/release/nvs.exe` predated the member, so the
  roster (`nvs meta --json`) did not list it. `--record-perf` rebuilt it, and `bun nv proofs
  --gate --only 'Core\Env::secret'` is green.
- The four `Core\Env` benches declared `calls 0` but measured 1 or 2 (the playbook's stale-count
  trap). `secret` and `context`/`mode` now declare `calls 1`, `get` `calls 2`, and the group's
  perf is recorded. `Core\Env::all` stays a known gap.
- The nightly's site failures (`covariant`, `union-types`, `properties`,
  `methods-and-inheritance`) ran on `afbb1edc2` and pass at HEAD.
- **Expected red at stage 13 (website questions):** `bun nv site --check questions` names six
  stale reference pages: `/reference/core/csrf/issue/`, `/reference/core/env/`,
  `/reference/core/session/`, and `start`, `regenerate` and `destroy` under it. They changed in
  stages 7 to 9. Stage 12 rereads each, rewrites its questions, and stamps it.

**Earlier fix.** `server-tests` was red on
`a_schedule_entrys_limits_and_grants_narrow_its_run_and_never_widen_it`
(`crates/nvs-server/src/schedule.rs:1498`): the test's fire now runs as a scheduled run, and
`bun nv verify --fast -p nvs-server` is green (`8d18b22c2`).

**Stage 8 (the CSRF form field), `3c3140d00`:**

- `nvs_server::route::csrf` (`crates/nvs-server/src/route.rs`) takes the token from
  `x-csrf-token`, else the `_csrf` query parameter (`TOKEN_FIELD`); the first present decides. With
  neither, a keyed checked urlencoded write answers the new `Csrf::InBody`, and anything else is
  refused. A multipart body is never read; `TOKEN_HEADER`'s doc says why.
- **The door cannot pull the body**: it runs inside the connection's poll, and a pull there is the
  deadlock `RequestBody::next_chunk` forbids. So `InBody` wraps the program in
  `route::reading_the_form`, and the request's own task calls `route::csrf_in_body` before the
  handler: it reads up to `ctx.request_body_limit()`, checks the field, and puts the bytes back
  with `set_buffered_body` so every body reader still finds the whole form. A refusal is an empty
  `403` with nothing run, or, in refusal mode, the program runs with `Refusal::Csrf` and no body.
  A body past the limit is refused as a missing token (403, not 413).
- Callers: `crates/nvs-cli/src/serve.rs` (after the verdict), `answer_on_the_wire` and
  `UnderTest::answer` in `crates/nvs-cli/src/runner.rs` (the in-process body is in memory, so it
  calls `csrf_in_body` directly).
- Tests, each passed once: `a_form_field_or_a_query_parameter_carries_the_token_and_a_multipart_body_is_never_read`
  and `a_refused_form_is_answered_before_the_handler_runs` (`route.rs`).
- Rule `security/csrf-is-on-by-default` gained the three places, what the read spends, and a
  Declined line for reading a multipart body. `Core\Csrf::issue`'s card names all three.
- **Expected red at stage 13:** `website/src/data/core.json:7504` still holds the old
  `Core\Csrf::issue` text, so a website staleness or reference-card check may name it. Regenerate
  it with the website's data command in stage 12.

**Stage 7 (sessions):**

- The `db` store is `crates/nvs-stdlib/src/session/sqlite.rs`: one `nvs_session` table in
  `<data>/session.sqlite`, WAL, `synchronous = NORMAL`, `busy_timeout = 5000`, one connection per
  core in a thread-local taken out for each statement. `load` reads only a row whose expiry is
  ahead; `save` is one upsert, and a due `sweep` (once a minute per core) deletes expired rows.
- `crates/nvs-stdlib/src/session.rs` picks it in `opened` where no `[session] backend` is written,
  and throws at `start` naming `[session]` where the data folder is unusable.
- The rule `http-server/no-session-block-means-no-store` is now
  `http-server/no-session-block-means-the-data-folder-store`; the backend, expiry and write-whole
  rules say what is true, and `bun nv rules --check` is green.
- Tests, each passed once this session: the three in `session/sqlite.rs`
  (`two_requests_of_one_session_both_finish_and_the_last_write_wins`,
  `a_record_survives_into_a_connection_another_process_would_open`,
  `an_expired_row_is_absent_and_the_sweep_deletes_it`), and four cases under `nvs test`:
  `session-start-with-no-block-opens-the-data-folder-store.nvst`,
  `session-start-takes-the-presented-identifier-or-none-at-all.nvst`,
  `a-nullable-core-parameter-keeps-the-mark-of-its-type.nvst` and
  `a-session-survives-a-request-on-another-process.nvst`.
- `session-start-refuses-a-tree-that-configured-no-store.nvst` is deleted: a bare `start` now
  opens the file. Any other test or case that expected `Core\Session::start` to throw with no
  `[session]` block goes red at stage 13 and should choose `backend = "shared"` with no address.

**Expected red at stage 13** (found, not fixed):

- `a_capability_query_answers_true_for_exactly_the_granted_names`
  (`crates/nvs-stdlib/tests/capability.rs:868`) expects `Core\Cap::has` to answer `true` for
  `fs.read` alone. Stages 3 and 4 opened `fs.write` and `net.connect` on a CLI run, so it now
  gets more. The test should write `write = []` and `connect = []`, or count the defaults.
- Stage 6 opened `process.exec` on a CLI run and `script.spawn` for the app folder. A Rust test or
  a `.nvst` case that expects a program refused on a CLI run with nothing written, or a spawn
  refused through `Authority` with nothing written, now gets the default. The process cases were
  checked (`process-exec-is-deny-by-default.nvst` is now
  `process-exec-written-as-an-empty-list-refuses-every-program.nvst`). Tests that ask
  `Capabilities::allows` directly are not affected.
- A test that asserts `Core\Cap::has('script.spawn')` or `process.exec` is `false` with nothing
  written now gets `true` on a CLI run.
- The acceptance check `config-tests` was red on stage 4's authority test. It is fixed and green
  (`fd07d523a`).

**Stage 6 (programs and scripts):**

- `defaults::capability` (`crates/nvs-config/src/defaults.rs`) gives `process.exec`
  `Granted::Everything` on a CLI run, and `script.spawn` the new `Granted::AppCode` on both run
  kinds: the app folder minus dot-paths and `nvs.toml`, and **minus the run's own temporary
  directories** (`Run::spawns_by_default`), so a request cannot write a script and spawn it.
  `Authority::allows` answers an unscoped ask `true` under `AppCode`: that is the method form.
- A bare name is `nvs_config::capability::bare_program` (no `/`, no `\` or `:` on Windows, not
  `.`/`..`), and `on_path` looks it up (`PATHEXT` on Windows, the execute bit elsewhere; a relative
  `PATH` folder is skipped). `anchor_program` writes a bare `exec` entry as its absolute path at
  load, and `silent_entry` refuses one `PATH` does not answer. **A folder entry needs a separator
  now**: `exec = ["target"]` in `nvs.toml` became `"target/"`, and the case
  `process-run-asks-for-the-capability-before-it-looks-at-the-target.nvst` writes `"allowed/"`.
- The exec door (`exec_with`, `crates/nvs-runtime/src/capability.rs`) asks the grant unscoped
  before a bare name is looked up, then asks `require` with the found absolute path.
  `spawn_target` is gone. **This session's reading of decision 6:** server work also looks a bare
  name up on `PATH`, but only once a written line exists, and the found path must pass that line.
  Matching the call against the entry's name instead fails once canonicalization resolves a
  symlinked entry (`python3` to `python3.12`). The rule records it.
- The compiler joined `Process::run("git")` to the file's folder, so the name never reached the
  door. `CoreTy::Program(Qual)` and `ParamText::Program` (`crates/nvs-stdlib/src/registry.rs`)
  mark `run` and `spawn`'s first parameter. `resolve_args` (`crates/nvs-types/src/paths.rs`) leaves
  a bare literal there as written and joins every other one.
- The hostile block writes `exec = []`. It does **not** write `spawn = []`: the `root = "."` block
  already grants `spawn = true` to every attack, so the default opens nothing new there, and `[]`
  would take a grant away.
- The `[[schedule]]` refusal (`crates/nvs-config/src/schedule.rs`) names the app folder.
- Tests, each passed once: `a_bare_name_is_found_on_path_on_a_cli_run_and_needs_a_line_in_server_work`
  and `a_script_in_the_app_folder_spawns_and_one_outside_it_does_not`
  (`crates/nvs-runtime/src/capability.rs`), `a_scheduled_script_in_its_app_folder_needs_no_spawn_grant`
  (`crates/nvs-config/tests/resolve.rs`), `a_bare_exec_entry_is_looked_up_on_path_when_the_configuration_loads`
  and a row in `a_grant_entry_that_allows_nothing_is_refused_naming_the_line_to_write`
  (`crates/nvs-config/tests/capability.rs`). Six process cases pass under `nvs test`.
- Rules rewritten: `security/process-exec-capability` (new title, three Declined lines) and
  `security/script-spawn-capability` (one Declined line). `docs/reference/tools/20-config.md`
  states both defaults.

**Stage 5 (SQLite and database TLS):**

- `sqlite_read_only` (`crates/nvs-stdlib/src/db/open.rs`) is the grant decision for a
  program-supplied path. `:memory:` and `""` ask nothing. A `file:` URI asks `db.open`, `fs.read`
  and `fs.write` as before. A path requires `fs.read`, and opens read-only where `fs.write` is not
  granted. `SqliteTarget::read_only` (`crates/nvs-db/src/sqlite.rs`) opens with
  `SQLITE_OPEN_READ_ONLY`. A read-only open's pool key is the settings key plus `:read-only`, and
  its pool bounds come from the plain key.
- A path-shaped `db.open` entry now grants nothing, so `silent_entry` refuses it at load, with
  help naming `write = ["<its folder>"]` under `[capabilities.fs]` (`Silent::under`).
- Postgres: `request_tls(stream, loopback)` gives `false` for `N` on a loopback address, and
  `open_stream` returns `PgStream::Loopback(NvsTcp)`. A loopback server that answers `S` is still
  verified. MySQL and MariaDB share `mysql::open_tcp`: on loopback it reads the greeting with
  `REQUIRED_OVER_A_SOCKET` (no longer `cfg(unix)`), and a greeting without `CLIENT_SSL` gives
  `MyStream::Loopback`.
- Tests: `a_sqlite_file_asks_the_file_rule_and_opens_read_only_where_it_is_not_writable` and
  `a_sqlite_open_asks_fs_read_before_it_reads_the_path` (`open.rs`),
  `a_read_only_target_reads_the_file_and_a_write_is_refused` (`sqlite.rs`),
  `a_loopback_server_with_no_tls_is_used_in_the_clear` (`pg.rs`),
  `a_loopback_server_that_offers_no_tls_is_used_in_the_clear` (`mysql.rs`). Each passed once.
- Cases: `db-open-asks-the-grant-about-a-sqlite-path-and-not-a-host.nvst` is renamed
  `db-open-asks-the-file-rule-about-a-sqlite-path-and-not-db-open.nvst`, and it and
  `db-open-refuses-a-tls-mode-weaker-than-verify-full.nvst` write `read = []`. Both pass under
  `nvs test`.
- Rules: `core-classes/db-capabilities` (the SQLite file rule, a Declined line) and
  `core-classes/db-safe-connection-defaults` (the loopback exception, a Declined line).

**Stage 4 (outbound hosts):**

- Two tests in `crates/nvs-ext/tests/http.rs` relied on the old defaults, and both now pass:
  the private-address case writes `internal = []`, and the second caller in
  `a_guest_waiting_on_an_http_response_lets_another_task_on_its_core_run` writes `connect = []`.
  That caller's call used to be refused; with `net.connect` open it was sent, nobody answered it,
  and `block_on` spun forever.
- Stage 4's full gate never finished, so stage 13 may find more tests and cases like these.

- `defaults::capability` (`crates/nvs-config/src/defaults.rs`) gives `net.connect`
  `Granted::Everything` on both run kinds. `connect_to` and `downgrade` keep `Nothing`.
- **Loopback is `defaults::internal(run)`**, beside `capability`, with
  `NET_INTERNAL_LOOPBACK = ["127.0.0.0/8", "::1"]`. It is not a `DEFAULTS` row: `Value::Startup`
  has no CLI arm, and a capability is not a value `Core\Config::get` renders. The rule's table names
  it as the fourth startup row. `Run` gained `development: bool` and `Run::in_development`;
  `Request::run` sets it from `started()`, which ignores the overlay, so a flip moves nothing.
- `Authority::address_refused` uses the written `internal` key when there is one, and otherwise the
  default list. A written `internal` replaces the default whole, loopback included.
- CIDR: `internal_range`, `masked` and `internal_covers` in `crates/nvs-config/src/capability.rs`.
  A range with host bits set (`10.0.0.5/8`) is refused at load by `silent_internal`. Only `::ffff:`
  forms are unmapped, so `::7f00:1`, NAT64, 6to4 and Teredo loopback stay refused by the default.
- The hostile block writes `connect = []` and `internal = []`. The three `docs/examples` Http blocks
  in `nvs.toml` write `internal = []`, because their outputs show loopback refused. 9 `.nvst` cases,
  `crates/nvs-cli/tests/fixtures/test-grants/denied.toml`, and the Rust tests in `db/open.rs`,
  `http.rs` (`GRANTED`), `net.rs` and the runtime's `reaching` write the empty keys.
- Tests: `a_public_host_needs_no_grant_and_loopback_is_open_on_a_cli_run_and_in_development` and
  `an_operator_exception_names_addresses_and_ranges_and_widens_nothing_else`
  (`crates/nvs-config/tests/capability.rs`), `a_flip_to_development_does_not_open_loopback`
  (`crates/nvs-config/tests/request.rs`), and
  `a_callers_default_hosts_are_still_narrowed_by_the_entry_and_the_manifest`
  (`crates/nvs-ext/tests/grants.rs`).
- Rules updated: `security/net-address-policy` (ranges, the defaults, a Declined line for loopback
  in production), `config/a-startup-default-is-never-flipped` (the fourth row, nine rows in all),
  `config/a-mode-is-five-defaults`. `docs/reference/tools/20-config.md` states the net defaults.

The nightly's proofs failures were the stage 3 restamp: `syntax/errors/limits.mdx` and the
questions of the capabilities and trust pages. They are restamped in this group.

**Stage 3 (files):**

- `nvs_config::defaults::capability` (`crates/nvs-config/src/defaults.rs:364`) returns
  `Granted::Everything` for `fs.read`/`fs.write` on a CLI run, and `Granted::AppFolder` (read) /
  `Granted::Temporary` (write) in server work. `Authority` applies them in `allows`,
  `allows_resolved`, `allows_host`, `allows_name` and `holds`. `holds` gives `Grant::Nothing` for a
  folder-shaped default, so an extension gets no files by default in server work.
- `Run` gained `temporary: &[PathBuf]` (the context's canonical temporary dirs) and
  `Run::reads_by_default` / `writes_by_default` (`crates/nvs-config/src/capability.rs`), and
  `Authority::with_temporary`. The runtime's `granted_with` and `allow`
  (`crates/nvs-runtime/src/capability.rs`) pass `ctx.temporary_dirs()`. `temp_dir` canonicalizes
  the root and asks with its list plus the new entry, so server work can create one.
- `nvs serve` prints one boot line per app folder whose tree writes no `fs.read`
  (`default_reads` in `crates/nvs-cli/src/serve.rs`).
- `nvs.toml` has the `[[app]] root = "tests/hostile"` block with `read = []` and `write = []`.
  Every case that relied on deny-all now writes the empty key: 16 `.nvst` cases, 3 hostile
  `nvs.toml`s, 4 example `nvs.toml`s/blocks, and the `io.rs` and `tests/capability.rs` helpers in
  `nvs-stdlib`. `docs/reference/tools/20-config.md` (the agent primer) states the new defaults.
- Tests: `server_work_reads_its_app_folder_and_writes_only_its_temporary_directories`
  (`crates/nvs-config/tests/capability.rs`) and
  `server_work_writes_its_own_temporary_dirs_and_not_another_requests`
  (`crates/nvs-runtime/src/capability.rs`).
- Rules updated: `config/no-configuration-file-is-a-complete-configuration`,
  `security/capability-question-is-grant-and-scope` and `core-classes/temporary-dir-sweep`.

**From stage 2:**

- **The app folder** is `nvs_config::app::folder(entry, keys)` (`crates/nvs-config/src/app.rs:699`):
  the most specific matching block key that is not the entry itself, else the entry's folder. An
  `entry` block is passed over, so it never shrinks the application; this is this goal's reading
  of the decision. `Snapshot::folder` (`snapshot.rs:163`) serves a CLI run and
  `Published::folder` (`snapshot.rs:530`, cached per path) serves a mount, a schedule and a job.
- **It is stored on `nvs_config::Request`, beside `serving`** (`request.rs:94`), not as a `Ctx`
  field. Every isolate constructor clones the configuration, so it inherits with no extra code, and
  `Ctx::set_config` carries it over (`wiring.rs`). Hosts write it with `Ctx::set_app_folder`
  (`crates/nvs-runtime/src/ctx/isolate.rs:108`) or `nvs_host::Isolate::in_folder`
  (`crates/nvs-host/src/isolate.rs:392`, applied beside `running_as`). Wired: `nvs run`
  (`main.rs`), `nvs test` and its per-test contexts (`runner.rs`), the shared worker
  (`shared.rs`), a served request from its mount entry (`crates/nvs-cli/src/serve.rs`, both the
  run and the rerun), a scheduled fire, and a job (`worker.rs` `configure`).
- **The defaults function** is `nvs_config::capability::Authority` (`capability.rs:303`) over a
  `Run` (`:258`), reading `nvs_config::defaults::capability` (`defaults.rs:364`). A written key replaces the default whole, and a header
  with no key leaves it. `Request::authority` builds one. `Run::default()` is server work with no
  folder, the narrowest run.
- **Every asker goes through it**: `capability::granted` and the resolved-path door `allow`, the
  address table (`Authority::address_refused`, where stage 4's loopback default goes), the
  schedule check (as server work with the script's folder), `nvs_ext::grants::Caller` (now
  `authority: Option<Authority>`, built in `crates/nvs-cli/src/extensions.rs` from a cloned
  `Request`), and the compiler's literal `db.open` and `queue.purge` checks in
  `crates/nvs-types/src/intrinsics.rs`, asked as a CLI run because that is the widest run.
  `nvs_config::store::reachable` still reads `allows_unscoped` directly: it counts written
  `cache.shared` grants, which keep no default.
- Tests: `crates/nvs-config/tests/snapshot.rs` (`a_mounted_entrys_app_folder_is_its_most_specific_root`,
  `a_cli_entry_and_a_publish_agree_on_the_app_folder`), `crates/nvs-config/tests/capability.rs`
  (`an_authority_answers_the_written_key_and_the_default_for_the_rest`), and
  `an_isolate_takes_its_trees_app_folder_and_run` in `crates/nvs-runtime/src/ctx/isolate.rs`.
  `crates/nvs-ext/tests/{grants,files,http}.rs` build their `Caller` with `Authority`.
- `rule:security/capability-question-is-grant-and-scope` now names the run as an input.

### Next

**No gate until stage 13.** Check with `cargo build` and `cargo clippy --all-targets -- -D
warnings`, commit each slice, and name in the handoff the tests and cases you expect to go red.

**Stage 12**: the website and the rules. Follow the stage's `**Split:**` line. The website pages
first: `website/src/content/docs/guides/how-to-use/configuration.mdx` and
`website/src/content/docs/guides/production/security-checklist.mdx`, then the other listed pages,
held to § *Text an end user reads*. Regenerate `website/src/data/core.json` (the
`Core\Csrf::issue` text at about `:7504`) with the website's data command, and reread and stamp
the six reference pages with stale questions (`bun nv site --questions --stamp <page>`). Then the
two package rules and the eight rules the stage lists, and `bun nv rules --check`. The examples
under `docs/examples/` already say what is true, so a website page can follow their wording.

**Proofs use the release binary.** `bun nv proofs --gate` reads the roster from
`target/release/nvs.exe`. After adding a `Core` member, pass `--nvs target/debug/nvs.exe` or run
`--record-perf`, which rebuilds it.

### Backlog

- `crates/nvs-stdlib/src/` holds many `default_*.profraw` coverage files from some earlier run. Find what writes them there and delete them. `crates/nvs-stdlib/src/csrf.rs:1`
- `Core\Queue::push`'s `grants` refusal still says ``help: grant it in nvs.toml under ...``: it is about the request's own grants list, not a missing grant line, and may adopt `grant_line` when stage 2 touches queue grants. `crates/nvs-stdlib/src/queue.rs:3085`
- `nvs-stdlib test ldap` failed 9 Samba cases in one full verify run. The cause was a TLS handshake that passed its 30s deadline, with the binary taking 205s, and it passed alone in 5s twice. If it repeats, find what loads Samba during verify. `crates/nvs-stdlib/tests/ldap.rs:49`
- Stage 12: six reference pages have stale questions (see *State*). `bun nv site --questions --stamp <page>` after a reread. `website/src/content/docs/guides/how-to-use/configuration.mdx:1`
- Stage 12: a website page that says a session needs a configured store, or that `start` throws with no `[session]` block, now says what is not true. `website/src/content/docs/guides/how-to-use/configuration.mdx:1`
- The editor offers file names at a `ParamText::Path` argument and not at `ParamText::Program`, so `Core\Process::run`'s first argument lost path completion, hover and links. `crates/nvs-lsp/src/completion.rs:743`
- The examples' IO folders' own `nvs.toml` files were not checked for keys a default now gives. `docs/examples/core/IO/read/about.md:1`
- `tools/config/network-grants-the-addresses-and-endpoints-they-reach/nvs.toml` lists `127.0.0.2` under `connect`, which passes only because its written `internal` replaces the loopback default. Check that its comment says so. `docs/examples/tools/config/network-grants-the-addresses-and-endpoints-they-reach/nvs.toml:1`
- The package half of every default is goal `namespace-grants`'s: a package is checked against its own line alone. `docs/goals/namespace-grants.md`
