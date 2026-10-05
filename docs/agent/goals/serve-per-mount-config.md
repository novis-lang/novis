---
milestone: post-parity
position: last
---
# Loop goal — every mounted entry runs under its own `[[app]]` blocks, and the door under the host's

`nvs serve` builds one configuration snapshot for the whole process, and every mount runs under it
(`crates/nvs-cli/src/serve.rs:37-43`, `:182`, `:299`). Started with a `<file>`, it folds the `[[app]]`
blocks that match that one file, and every other mount runs under them. Started with no file, it folds
no block at all, and every mount runs under the global tree alone (`nvs_config::Snapshot::host`,
`crates/nvs-config/src/snapshot.rs:151-165`; `rule:http-server/the-served-file-is-optional`). So a
second application's `[app.limits]`, `[app.capabilities]`, `[app.log]`, `mode` and `origin` are
silently ignored, or it runs with the first application's grants. That is a request-isolation defect,
priority 1. [ADR 0200](../../decisions/0200.md) § *Revisiting* names this exact change.

This goal makes the snapshot a request runs under the one its **own entry file's** `[[app]]` blocks
fold, as `nvs run <file>` already does, and leaves what the door decides before a mount is selected on
the host's snapshot:

```text
door      [server], [http.headers], [http.cors], trusted_proxies, admission, waits   host snapshot
handler   [http] csrf_key, [session] cookie, [trace] sample, the mount table         host snapshot
program   [limits], [limits.hard], [capabilities], [log], mode, origin, Core\Config   its entry's snapshot
```

## Why here

**Directly after goal `performance-pass`, by the user's decision of 2026-10-05.** It is a priority-1
defect, and every goal behind it is tooling or extensions, so nothing behind it should run first. Goal
`ext-grants` computes a guest's rights as the intersection of the entry, the manifest and the
*caller*, and the caller's rights are the request's `[capabilities]`: that intersection is only right
once a request holds its own application's grants. Goal `performance-pass` runs first because it is
live and measures the request path this goal then changes by one lookup.

**What this spends:** one snapshot per distinct chain of matching `[[app]]` blocks per publish
(kilobytes each, bounded by the configuration and not by traffic), one map entry per entry file
resolved under that publish (bounded by the files on disk, as the compiled-unit cache already is), and
two publishes alive during a swap. Per request: at most one map lookup and one `Arc` clone, no syscall
and no allocation. `rule:config/the-config-is-an-immutable-snapshot`'s cost paragraph is rewritten to
say so.

## Stage 0 — the catch-up

The sentences on disk this goal makes false, each rewritten whole by the session that lands the
behaviour:

- `docs/rules/http-server/the-served-file-is-optional.md` — the table's third column and the bold
  paragraph "One snapshot serves the process" (Stage 2).
- `crates/nvs-cli/src/serve.rs:37-43` (module doc), `:1989-1994` (`fall_back_to`'s "per-application
  gap"), `crates/nvs-config/src/snapshot.rs:3-6`, `:151-158`, `:359-366`,
  `crates/nvs-server/src/serve.rs:707-719` ("one tree for the whole instance"),
  `crates/nvs-cli/src/control.rs:143-146` (Stages 2 to 4).
- `docs/rules/config/a-scheduled-run-is-a-root-isolate.md:7-10` and
  `docs/rules/concurrency/a-job-runs-as-a-root-isolate.md` (Stage 5).
- `docs/reference/tools/25-server.md:22-29` and `docs/reference/tools/20-config.md:321` (Stage 6).
- ADR 0200 is frozen history and is not edited; the new record `modifies` its rule.

## Stage 1 — the floor

Nothing is carried. The driver deletes goal `performance-pass` when this goal starts. The suites, the
`.nvst` trees and `nv verify` are the floor. A process whose configuration writes no `[[app]]` block
serves exactly what it served before this goal, byte for byte.

## Stage 2 — the record and the published set

**Does:** Writes the decision record, and makes one publish hold the host's snapshot and one snapshot
per entry file, built from the same tree.

One file set: `crates/nvs-config/src/snapshot.rs`, `crates/nvs-config/src/app.rs`,
`crates/nvs-config/src/control.rs`, `crates/nvs-config/tests/snapshot.rs`,
`docs/rules/http-server/the-served-file-is-optional.md`.

- **The decision record**, written first, for the whole goal, from § *Standing decisions*. It
  `modifies` `http-server/the-served-file-is-optional`, `config/the-config-is-an-immutable-snapshot`,
  `config/a-scheduled-run-is-a-root-isolate` and `concurrency/a-job-runs-as-a-root-isolate`. It states
  the tradeoffs from *What this spends*.
- **The published set.** `nvs_config::Current` (`snapshot.rs:366`) holds one value per publish: the
  host's snapshot, and the way from an entry file to its own. `Current::load` keeps returning the host's
  snapshot, so its many callers (`control.rs`, `worker.rs`, `Serving`) do not move; a second method
  returns the publish. Host and entries are swapped under the one lock, so no request reads two
  publishes.
- **One snapshot per chain of blocks.** An entry's snapshot depends only on which blocks match it
  (`app::matching`, `app.rs:436`) and in what order, so entries that match the same chain share one
  `Arc`. The fold is `Snapshot::folded`'s (`snapshot.rs:169-245`), run **over the host snapshot after
  its `Boot` carry** (`carry_boot`, `snapshot.rs:317`), so a `Boot` key reads the running value in
  every snapshot of the publish. An entry no block matches gets the host's `Arc` itself.
- **Matching runs once per entry per publish**, never per request (`app.rs:41-42`): eagerly for every
  mounted entry when the publish is built, so a block that does not fold refuses the boot or the reload
  (`E0601`); lazily, and then cached in the publish, for a file first compiled later — a `dispatch =
  "path"` file (Stage 3) or a scheduled or queued script (Stage 5).
- **Pinned by** the Stage 2 check, in `crates/nvs-config/tests/snapshot.rs`.

## Stage 3 — the request path

**Does:** Runs each request under the snapshot of the entry it selected, while the door keeps
answering under the host's.

Two file sets, in this order. The door: `crates/nvs-server/src/serve.rs` (`Reply` at `:422`, the
request start at `:1427-1428`, `set_config` at `:1670`, `disconnect_for` at `:1680`). The binary:
`crates/nvs-cli/src/serve.rs` (boot at `:182-403`, the handler at `:922-1102`, `fall_back_to` at
`:1995`), `crates/nvs-config/src/server.rs:840-880`, and a new `crates/nvs-cli/tests/per_mount_config.rs`
built on `crates/nvs-cli/tests/live_config.rs`'s `Server` harness.

- **`Reply::Run` carries the snapshot the program runs under.** `nvs-host` names no configuration
  crate, so the isolate cannot carry it; `nvs-server` already does. The handler picks it after
  `Table::select` (`crates/nvs-cli/src/serve.rs:937`) from the file it is about to run, and the door
  writes that one onto the context at `:1670` and reads the disconnect keys from it at `:1680`.
  Everything above `:1576` stays on the host's snapshot (§ *Standing decisions*).
- **`Serving::policy` (`crates/nvs-server/src/serve.rs:809`) is only ever handed the host's
  snapshot.** It re-derives on a pointer change, so a per-mount snapshot there would re-derive and
  write the policy lock on every request that alternates mounts.
- **Each row's origin is its own entry's.** `fall_back_to` (`crates/nvs-cli/src/serve.rs:1995`)
  takes each row's `[[app]] origin` from that row's snapshot, not from the boot's.
- **Admission's per-request cap is the largest any snapshot of the publish can reach**
  (`per_request_cap`, `crates/nvs-config/src/server.rs:874`): the global `[limits]` and every
  `[[app]]` block's, which the roster gives without an entry. The ceiling is process-wide and only
  ever gets smaller from this.
- **The process cache tier needs no change.** It already scopes an entry by the request's own most
  specific block and its generation (`crates/nvs-stdlib/src/cache.rs:1515`); today two mounts share a
  scope because they share a snapshot. The check proves they no longer do.
- **Pinned by** the Stage 3 checks. The attack is
  `a_mount_cannot_use_a_capability_only_another_applications_block_grants`: two mounts, one `[[app]]`
  block granting `process.exec` and an `fs.read` root to the first; the second mount's program tries
  the call, tries `Core\Config::set` on the grant, and `spawn script`s a file under the first
  application's root. Every attempt is refused, and the first mount's call still works.

## Stage 4 — reload and the mount rows

**Does:** Rebuilds every entry's snapshot on a reload and on a rescan that adds a row, and folds each
row's own `[[app]] origin`.

One file set: `crates/nvs-cli/src/control.rs` (`entry` at `:143-146`, `resolved` at `:406-420`, the
publish at `:476`), `crates/nvs-cli/src/serve/mounts.rs` (`Rescan::new` at `:323`, `pass` at
`:354-367`), `crates/nvs-config/src/control.rs`, `crates/nvs-cli/tests/per_mount_config.rs`.

- **A reload builds the whole publish again**, host and entries, from the resolved tree. The named
  `<file>` no longer selects anything for the reload; `Process::entry` goes.
- **A row a rescan adds** is resolved to its snapshot when it is compiled (`mounts.rs:374`), in the
  publish standing then. A reload that moves any `[[app]]` key rebuilds every row's origin.
- **Pinned by** the Stage 4 check.

## Stage 5 — scheduled fires and queued jobs

**Does:** Runs a `[[schedule]]` fire and a queued job under the `[[app]]` blocks that match its own
script.

One file set: `crates/nvs-cli/src/serve.rs` (`Scheduled::isolate` at `:1738-1781`),
`crates/nvs-cli/src/worker.rs` (`:360`, `:625`), `crates/nvs-server/src/schedule.rs:1180`, and the two
rule fragments.

- A fire sets its context from the publish's snapshot for `entry.script()`, not from `Current::load`
  (`crates/nvs-cli/src/serve.rs:1746`). A job does the same for its script at `worker.rs:625`. The
  worker's own context at `:360` keeps the host's.
- **Pinned by** the Stage 5 check.

## Stage 6 — the reference and the proofs

**Does:** Rewrites the reference tables, the two `about.md` files and the help for what a served entry
runs under, and adds the attack.

- `docs/reference/tools/25-server.md:22-29`: the table's right column says every mount runs under the
  blocks that match its own entry. `docs/reference/tools/20-config.md:321`: the same sentence for
  `nvs serve` beside `nvs run`.
- `docs/examples/tools/server/nvs-serve/about.md` and
  `docs/examples/tools/config/app-per-application-blocks/about.md`, each inside its word band, and the
  `nvs serve --help` text if it names the `<file>`'s role.
- **The attack** in `tests/hostile/tools/config/app-per-application-blocks/`, which runs under `nvs
  run` with that directory's `nvs.toml`: a program covered by one block uses a right that only a
  sibling directory's block grants, sets it with `Core\Config::set`, and spawns a script that lives in
  that directory. Every attempt should fail. The served, two-mount form of the attack is Stage 3's Rust
  test, because a hostile case answers one request under one program.
- `bun nv reference` regenerates `docs/novis.md`.

## Standing decisions

- **The split: per mount is what an `[[app]]` block may write; process-wide is everything else.** An `[[app]]` block holds `root`/`entry`, `mode`, `origin`,
  `[app.limits]`, `[app.limits.hard]`, `[app.capabilities]` and `[app.log]`
  (`crates/nvs-config/src/snapshot.rs:220-225`, `rule:config/an-application-is-its-entry-file-path`),
  and those are read per request off the context's snapshot. Every other block is read once for the
  process, or by the door before a mount is selected, from the host's snapshot:
  - **Per mount:** `[limits]` and `[limits.hard]` (`Ctx::set_config`,
    `crates/nvs-server/src/serve.rs:1670`); `cancel_on_disconnect` and `disconnect_grace` (`:1680`);
    `[capabilities]` (`nvs_runtime::capability`); `[log]` (`crates/nvs-config/src/log.rs:114`);
    `mode` as `Core\Env::mode` reads it (`crates/nvs-config/src/request.rs:209-230`); `origin`
    (`crates/nvs-cli/src/serve.rs:1995`); `Core\Config::get`; the process cache tier's scope.
  - **Process-wide:** `[server]` `listen`, `workers`, `socket_mode`, `watchdog_margin`
    (`crates/nvs-cli/src/serve.rs:215-333`, `Boot`-class); `max_in_flight` and the admission arithmetic
    (`:235`); waits and connection bounds (`:211`, `:224`); `trusted_proxies`, `[http.headers]`,
    `[http.cors]` (`Serving::policy`, `crates/nvs-server/src/serve.rs:809`, answered before a mount
    exists); `dispatch`, `static`, `health_path` and the mount table (`switches_for`,
    `crates/nvs-config/src/server.rs:768-773`, which already says no block is consulted); `[opcache]`
    (`crates/nvs-config/src/cache.rs:509-511`, the same); `[http.client.tls]` (one TLS client,
    `crates/nvs-cli/src/serve.rs:197`); `[control]`; `[trace] sample`, `[http] csrf_key` and the
    session cookie name (`:1025`, `:1052-1056`); `[mode] default` and `ceiling` as startup values;
    `[[schedule]]`, `[queue]` and `[db.*]`.
  - **Fallback:** a key a session finds that neither list names is process-wide until a record says
    otherwise, because a value read before the mount is selected cannot be per mount.
- **One generation per publish.** Every snapshot of one publish carries the host's `generation`
  (`snapshot.rs:182`). The database pool keys on it (`crates/nvs-runtime/src/pool.rs:137-169`), so a
  server with many applications keeps one pool per `[db.<name>]` as today, and the drain
  (`crates/nvs-cli/src/control.rs:481`) keeps reading one number. The cache tier stays distinct per
  application because its key also names the block. Fallback, if a test shows a pooled connection
  carrying state across applications: one pool per application, and the record says what it spends.
- **The named file chooses no configuration.** `nvs serve <file>` over a mount table still requires
  the file to be in the table (`crates/nvs-cli/src/serve.rs:1954-1965`), and that is now its only
  role. ADR 0200 § 4 becomes the ordinary case for the door.
- **A file is matched as itself.** Under `dispatch = "path"` the file the request runs is the entry
  whose blocks apply, not the row's `entry`, because an application is its entry file path. A spawned
  script runs under its parent's snapshot, as it does now (`Ctx::isolate`), and never under the blocks
  of the file it spawns.
- **No new key and no new diagnostic code.** A block that does not fold for one entry is the existing
  `E0601`, raised at boot or reload.
- **`nvs ctl config` reports the host's snapshot.** A per-application view on the control socket is not
  in this goal; a session that needs one puts it in the handoff's `## Backlog`.
- **One ADR slot**: one new record, and no number named here. It states the tradeoffs. Performance: one
  lookup per request. Memory: *What this spends*. Usability: a host of many applications gets each
  application's own policy with no file named. Simplicity: the named file loses its second meaning.
- **Every comment in a changed `.nvs` and every changed `about.md` follows `AGENTS.md` § *Text an end
  user reads* at the first write.**
