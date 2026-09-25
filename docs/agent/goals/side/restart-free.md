---
milestone: post-parity
---

# Side goal — a running server takes every code change without a restart, and every config change it can

When this goal is green, `nvs serve` never has to be restarted or reloaded for a source change. That
holds in production as well as development: an edited, added or removed `.nvs` file reaches the
next request. That covers an entry file, a `require`d file, an autoloaded class, a module a
discovery query or a mount `scan` finds, and the code a queue job or a scheduled fire runs. A
deploy of many files, fast or slow, is never compiled half done. A configuration file that is
saved is applied by the server itself. Exactly three keys need a restart — `[server] listen`,
`[server] socket_mode` and `[server] workers`. Each says so on its own line of the shipped
`nvs.toml`, and the server logs one as a pending restart the moment it changes.

This is the user's rule, and it is the whole of the goal: **a restart or a reload is never required
unless it is technically impossible without one.** PHP never asks for a restart after a source
edit, and a server that does is a production outage waiting for the deploy that forgets it.

## Why a side goal

It needs nothing the chain has not built, and nothing on the chain waits for it. Its files — the
unit cache, the serve loop, the configuration crate — are ones the chain's live goals do not open,
so it runs beside the chain in a worktree of its own instead of queueing behind ninety generated
goals.

## What is on disk today, measured

Every fact below was checked live against the release binary with a probe program before this goal
was written. Each stage's first item confirms it again before changing it.

- **Production never sees a code change.** `validate = "never"` is production's default
  (`crates/nvs-cli/src/script.rs`, step 1 of `Compiler::compiled`), and `nvs ctl reload` does not
  help: it reports `invalidated: 0` and the old code keeps serving. A restart is the only deploy.
- **Development sees only the entry file.** The in-memory unit key hashes the entry file's content
  alone (`Compiler::key`). An edit to an autoloaded class or a `require`d file is invisible until
  the entry file changes, and then everything recompiles at once. The on-disk artifact key already
  covers every file (`crate::cache::program_digest`), so a restart is always correct.
- **A discovery query's directories are not watched** (`rule:packaging/autoload-probes-fold-into-the-cache-key`
  says its half is not on disk).
- **The mount table is built once**, at boot (`crates/nvs-cli/src/serve.rs`, `table_for`), so a new
  module under a `scan` needs a restart. `docs/reference/tools/25-server.md` says a reload re-expands
  it, and that is false today.
- **A reload reports keys as applied that it does not apply**: all of `[opcache]`, `[http.headers]`,
  `[http.cors]`, the admission ceiling derived from `limits.memory`, the `[[schedule]]` roster,
  `queue.visibility` and everything a queue job runs under (the worker holds the boot snapshot,
  `crates/nvs-cli/src/worker.rs`), `[metrics]`, the `[trace]` exporter, and `[[app]] origin`.
- **Nothing notices a changed configuration file.** A reload is only ever pushed by an operator.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong. Each is rewritten whole by the session that lands the
behaviour, and not before:

- `docs/rules/config/an-edit-reaches-the-next-request-without-a-restart.md` — its five steps put
  the `stat` on the request path and key a unit on one file.
- `docs/rules/config/opcache-revalidation-is-system-class.md` and
  `docs/rules/config/a-startup-default-is-never-flipped.md` — `validate = "never"` and its production
  row; `dispatch` and `static` stop being `Boot`.
- `docs/rules/config/reloadability-is-its-own-field.md` — the `Boot` list.
- `docs/rules/config/the-config-is-an-immutable-snapshot.md` — "a reload is pushed by the operator".
- `docs/rules/config/a-reload-names-what-it-could-not-apply.md` — it also covers a change the
  server applied by itself.
- `docs/rules/packaging/autoload-probes-fold-into-the-cache-key.md` — the discovery half.
- `docs/reference/tools/25-server.md` (the `[server]` block's "read once, at start", the mount
  `scan` bullet, § *nvs ctl*) and `docs/reference/tools/20-config.md`.
- `crates/nvs-config/src/default.toml` — `[opcache]`'s "`never` is the production answer" and the
  `file_cache_dir` "applies at boot" note, and every key line Stage 7 marks.
- The doc comments in `crates/nvs-config/src/directive.rs` for every row whose apply class changes,
  and `crates/nvs-server/src/serve.rs`'s comment calling `[metrics]` Boot-class.

`docs/novis.md` and `docs/ground-rules.md` are generated and are regenerated, never edited. A
record under `docs/decisions/` is frozen; the new records' `changes: modifies` says what they
overtake.

## Stage 1 — the floor

Main's carried floor, which a side run is always checked against. Never traded.

## Stage 2 — every file a program reached is watched, in every mode (the keystone)

One file set: `crates/nvs-cli/src/script.rs`, `crates/nvs-config/src/cache.rs`,
`crates/nvs-config/src/default.toml`, and the resolver that runs a discovery query
(`bun nv peek --locate implementing`).

- **The unit is keyed on the program, not on its entry file.** A compile records every file it
  read, with the stamp and digest it read, and the unit key is the whole-program digest the disk
  cache already computes. A check of a unit checks every one of those files, the autoload probes
  (misses included, as today), and every directory a discovery query listed.
- **`validate = "never"` is removed.** `mtime` is the default in both modes and `hash` stays as the
  stricter one. A configuration that still writes `never` does not load, and its diagnostic names
  the two values that remain.
- **A deleted file** that a program still reaches makes that program's next compile fail. Its
  requests fail loudly (`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`, kept as it
  is).
- **A reverted edit** is answered from the unit already compiled for that content, not recompiled.

## Stage 3 — the check leaves the request path, and a deploy is compiled once, whole

One file set: `crates/nvs-cli/src/script.rs`, `crates/nvs-cli/src/serve.rs`,
`crates/nvs-config/src/cache.rs`, `crates/nvs-config/src/directive.rs`.

- **No request makes a file-system call to revalidate.** A background task on the compile pool
  checks every loaded program once per `revalidate_freq`. On a change it compiles the new program
  and only then swaps the pointer, so no request waits for a recompile of a file it already had. An
  idle server picks the change up within `revalidate_freq` plus the settle time, with no request
  needed to trigger it.
- **The settle time: a program is compiled from a quiet tree.** A new directive, `[opcache] settle`,
  is the time no file of the program may have changed before it is compiled. After the compile,
  every file is checked again, and a compile during which any file changed is thrown away and
  retried once the tree is quiet. A fast deploy (`git pull`, `rsync`, an unzip) is compiled once,
  from the finished tree.
- **An atomic deploy is atomic.** At the start of each compile, the program's root directory is
  resolved through every symlink or junction once, and every file is read through that real
  directory. After the compile, the root is resolved again, and a different answer means a
  recompile. A deploy that switches a `current` link from one release to the next can therefore
  never produce a program built from both.
- **Memory stays bounded.** A swapped-out unit is freed once the last request holding it ends. A
  server that has seen ten thousand edits holds units in proportion to its files, never to its
  edits (`rule:programs/memory-priority`).

## Stage 4 — the mount table follows the disk

One file set: `crates/nvs-cli/src/serve.rs`, `crates/nvs-config/src/mount.rs`.

- A `scan` is expanded again under the same background check, by listing each scanned directory
  when its stamp moves. A new module is compiled, has the boot-time origin check run on it, and is
  served. A removed module answers `404`. A new module that does not compile fails its own
  requests and nothing else.
- An explicit mount's entry file that appears or disappears is the same case.
- A path is still never derived from a URL (`rule:http-server/a-path-is-never-derived-from-a-url`):
  the table is still enumerated from the configuration's globs, only more than once.

## Stage 5 — every reloadable key really reloads

One file set: `crates/nvs-cli/src/serve.rs`, `crates/nvs-cli/src/worker.rs`,
`crates/nvs-cli/src/control.rs`, `crates/nvs-cli/src/script.rs`, `crates/nvs-server/src/`
(`secure.rs`, `cors.rs`, the admission ceiling, `otlp.rs`), `crates/nvs-runtime/src/metrics.rs`.

- `Secure` and `Cors` are derived once per published snapshot, not once per process.
- The admission ceiling is recomputed when a snapshot is published.
- The unit cache reads `[opcache]` from the published snapshot. A changed `file_cache*` swaps the
  artifact cache for new compiles, and units already in memory are untouched.
- The `[[schedule]]` roster is compared with the new one on publish. A fire already running
  finishes, and the new roster arms from the next tick.
- A queue worker takes the published snapshot for each job it claims, and `visibility` with it.
- The metrics registry, the scrape socket, the push endpoint and the trace exporter are rebuilt
  on publish, with a new socket bound before the old one closes.
- `[[app]] origin` is folded into the mount rows again on publish.
- **A census makes this permanent.** Every directive has a live proof that changing it takes
  effect in a real request, or a proof that it is reported as needing a restart. A directive
  added without one fails the census. This is the test that would have caught every key in
  § *What is on disk today*.

## Stage 6 — the configuration applies itself, and only three keys need a restart

One file set: `crates/nvs-cli/src/control.rs`, `crates/nvs-cli/src/serve.rs`,
`crates/nvs-config/src/directive.rs`, `crates/nvs-config/src/snapshot.rs`,
`crates/nvs-config/src/control.rs`, and each subsystem a key below moves.

- **The server checks its configuration files.** A timer off the request path checks the stamp
  of every file the boot resolved, every few seconds. A change is resolved, validated and
  published through the same function `nvs ctl reload` uses. A file that does not validate is
  logged with its line, and the running configuration stays. `nvs ctl reload` stays as the way to
  apply a change at once.
- **A pending restart is loud.** A changed restart key is logged by name with its running and its
  written value, the moment the change is seen. `nvs ctl status` lists every pending one until the
  process restarts.
- **`Boot` shrinks to three keys.** The `server` row is split into one row per key. Every key but
  `listen`, `socket_mode` and `workers` applies live: the timeouts, `max_in_flight`,
  `trusted_proxies`, `health_path`, `dispatch`, `static`, `root`, the mounts and `[server.connection]`.
  So do `[session]`, `cache.shared`, `http.client.tls`, `io.temp_root`, `opcache.file_cache_dir`,
  `[control] socket` and `[queue]` `connection` and `workers`. The new value is built, requests and
  jobs already running finish on the old one, and the old one is closed after them. A changed
  worker count starts or stops workers after their current job.

## Stage 7 — the template and the reference say it, and the proofs exist

One file set: `crates/nvs-config/src/default.toml` and its guards, `docs/reference/tools/25-server.md`,
`docs/reference/tools/20-config.md`, `docs/examples/config/`, `tests/hostile/`, `benches/members/`.

- **Every restart key's line in the shipped `nvs.toml` ends `# restart required`**, and no other
  line says it. A guard checks the template against the registry in both directions.
- **The reference gets one section, "What reaches a running server"**: a code change always, a
  configuration change by itself, and the three restart keys. It also says how to deploy: a fast
  copy is safe, and a slow upload should switch a link to a finished directory, which is always
  safe. It says what no compiler can make consistent: data written by old code and read by new
  code (cache entries, sessions, queued jobs).
- The section is a feature on the roster, so it owes its proofs: `bun nv proofs --id`
  names them.

## Standing decisions

These are the user's calls, made on 2026-09-23. No session re-decides one.

- **A source change never needs a restart or a reload, in any mode.** Where a design choice makes
  this harder, this rule wins over latency, simplicity and memory (priorities 3 to 5), and never
  over security (priority 1).
- **A configuration change is applied by the server itself.** `nvs ctl reload` remains, for applying
  at once.
- **Restart-only are `[server] listen`, `[server] socket_mode` and `[server] workers`, and nothing
  else.** A port below 1024 needs privileges the process has dropped, so `listen` cannot move in
  general. `workers` stays restart-only by the user's choice: each core holds its own runtime
  state, and resizing that live is a large change for a key set once per machine.
- **A deploy that does not compile fails the requests that reach it, in every mode.** The last good
  version is never served in its place (`rule:config/a-broken-edit-fails-the-requests-that-resolve-it`
  stands).
- **`[opcache] settle` is a directive**, `System`-class and reloadable, defaulting to `"1s"` in
  production and `"100ms"` in development.
- **`validate = "never"` is removed**, not deprecated.
- **`SIGHUP` still stops the server.** Reloading on it is not reopened: nothing needs a reload any
  more.
- **Two new decision records, and no other number**: one for source revalidation (Stages 2 to 4),
  one for configuration apply (Stages 5 and 6). Each takes the next free number on `main` at the
  moment it is written, and states the tradeoffs. Performance: one `stat` per loaded file per
  `revalidate_freq`, in the background, and none on the request path. Memory: a file list per unit,
  and two units alive during a swap. Usability: no deploy step. Simplicity: `never` is gone and the
  restart list is three keys.
- **Every comment in a new `.nvs` and every new or changed `about.md` follows `AGENTS.md`
  § *Text an end user reads* at the first write**, and `bun nv proofs --comments <paths>`
  is run over them before the wrap.
