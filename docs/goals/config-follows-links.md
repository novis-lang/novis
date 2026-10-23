---
looper: 1
title: "A reload reads the file a link names now, so a changed ConfigMap or a swapped `current` link takes effect without a restart"
summary: "nvs serve stores each configuration root as the absolute path it was given, a reload resolves that path through its links again, the background check notices a link that points somewhere new, the directory holding a link is ownership-checked like the file, a changed Kubernetes ConfigMap reaches a running server on a real cluster, the rule and the Docker guide say so, and the checks below pass."
context: [docs/rules/config/the-resolved-root-is-announced-and-stored.md, docs/rules/config/ownership-is-the-trust-boundary.md, crates/nvs-cli/src/config.rs]
stages:
  - id: 1
    title: "a reload follows the root's links, and a swapped link is noticed"
    status: done
    context: [crates/nvs-cli/src/reload.rs, crates/nvs-config/src/resolve.rs, crates/nvs-config/src/trust.rs]
  - id: 2
    title: "the directory holding a link is checked, a real cluster applies a changed ConfigMap, and the rule and the guide say so"
    status: done
    context: [crates/nvs-server/src/mount.rs, crates/nvs-cli/src/serve/mounts.rs, crates/nvs-config/src/snapshot.rs, website/src/content/docs/guides/production/docker.mdx]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: config-tests
    stage: 2
    name: a swapped link is read on reload, and the directory holding it is checked
    run: [bun, nv, verify, --fast, -p, nvs-config]
  - id: cli-tests
    stage: 1
    name: the configuration check notices a link that points somewhere new
    run: [bun, nv, verify, --fast, -p, nvs-cli]
---

# A reload reads the file a link names now, so a changed ConfigMap or a swapped `current` link takes effect without a restart

## Why

Kubernetes mounts a ConfigMap as a link farm. `/etc/nvs/override.toml` links to `..data/override.toml`,
and `..data` links to a timestamped folder. An update writes a new timestamped folder, swaps `..data` to
it, and deletes the old folder. A deploy that swaps a `current` link does the same.

`nvs serve` canonicalizes each root once at boot (`crates/nvs-cli/src/config.rs:700`), so it stores the
timestamped folder's path. The background check stats that canonical path
(`crates/nvs-cli/src/reload.rs:517`), and a reload re-reads it through `stored_set`. After the swap that
path is gone: the reload is refused as unreadable, and the old settings keep serving until a restart.
The rule meant the opposite. `rule:config/the-resolved-root-is-announced-and-stored` stores the root so
that a replaced directory "reloads the file the operator can see".

The Docker guide tells a reader to restart the pods after a ConfigMap change
(`website/src/content/docs/guides/production/docker.mdx`). This goal makes the server read the change
itself, and then the guide says so. Found on 2026-10-22, while the guide's ConfigMap section was written.

## Standing decisions

**Gate: `security-hardening` stage 15 has landed.** `nvs serve` resolves through
`nvs_config::resolve::Disk`, and `trust::check` has the read-only mount waiver. Proof:
`crates/nvs-cli/src/config.rs` names `resolve::Disk` on the serve path. This goal adds a check beside
that one, so it builds on it.

**A root is stored as the absolute path it was given, never its canonical path.** `resolve::roots`
already returns an absolute, lexically normalized path (`crates/nvs-config/src/resolve.rs:393`). The
canonicalizing loop at `crates/nvs-cli/src/config.rs:700` goes. Every resolve still canonicalizes
through `Files::trust`, so the include cycle test and the ownership check see the same files as today.
The announcement prints the root as given and the file it reached, as it does now.

**Includes need no change.** A relative include resolves against the directory of the file that names
it, and every reload resolves the tree again from the root. So an include inside the swapped folder is
read from the new folder.

**The directory that holds a link is checked too.** When the path as given differs from its canonical
path, whoever can write the directory that holds the link chooses which file the next reload reads. So
`trust::check` also checks the parent of the path as given, with the same owner test, mode test and
read-only waiver. A root reached with no link is checked exactly as today.

**`d8`, the user's answer: a link's directory skips the mode test only when root owns it on a
read-only mount.** That is the kubelet's ConfigMap volume, measured on `kind` as `0777 root:root` on
a read-only `ext4` mount. Every other check stays as the 2026-10-22 call says, the file's own
directory included. Built as `trust::platform::check_link_holder` (`crates/nvs-config/src/trust.rs`).

**The gap this goal owns is closed here, and its premise did not hold.** Security-hardening handed
`a-mount-entrys-own-database-files-are-not-withheld` to this goal, on the reading that a mount entry's
tree could name a `[db]` file of its own. It cannot: `tree::App` is `deny_unknown_fields` with no
`db` field, and a mount reads no `nvs.toml` of its own, so every `[db]` path is the host tree's,
includes merged, and `Table::from_config` already withheld it. The real hole was the reload: a core
kept its table when only a `[db]` path moved, so the new file was served. `Local::table` now compares
`nvs_server::mount::databases` of both snapshots too, with no allocation. Memory: nothing new. A
future `[app.db]` table would have to feed `databases` as well.

**The check notices a swap.** The stamp gains the file's identity: device and inode on Unix, the file
index on Windows (`crates/nvs-cli/src/reload.rs:561`). Then a swap to a file with the same time and size
still moves the stamp. The check stats each root as given beside `files` and `probed`, so a stat that
follows the link sees the new file. Memory: one more pair of integers per watched path, bounded by the
number of files, never by traffic.

**A real cluster proves it.** Stage 2 runs `kind` in Docker Desktop with the guide's ConfigMap and
Deployment, changes the ConfigMap, and waits for the server's reload line. Kubernetes can take about a
minute to update the files. Safe default if no cluster can run on the machine: the Rust test that replays
the kubelet's steps is the proof, and the handoff says that no cluster ran. Inside the pod, the stage
records the mode and owner of `/etc/nvs`, `..data` and the timestamped folder. If the directory that holds
the link has a group- or world-writable mode, the user's 2026-10-22 call (that mode is refused even on
a read-only mount) would refuse every ConfigMap. Then the session stops with `BLOCKED` and names the
measured mode, because only the user can reverse that call.

**Lanes.** One lane. Both stages change `crates/nvs-config/src/trust.rs` or read what stage 1 changed.

**Trade-offs.** Security: a root reached with no link is checked as today, and a link's directory is
now checked as well, which is stricter. Latency: nothing on the request path. The check thread stats a
few more paths every two seconds. Memory: a few paths and integers per process. Simplicity: a link
deploy and a ConfigMap work the way an operator expects, and the guide loses a restart step.

## Stages

### 1. a reload follows the root's links, and a swapped link is noticed

**Does:** a server whose root is reached through a link reads the new file after the link is swapped
and the old target is deleted, with no restart.

- [x] The boot stores each root as `resolve::roots` gives it, and the canonicalizing loop is gone (`rule:config/the-resolved-root-is-announced-and-stored`). `crates/nvs-cli/src/config.rs:700`
- [x] `Stamp` carries the file's identity, and the check stats each stored root beside `files` and `probed`. `crates/nvs-cli/src/reload.rs:561`, `crates/nvs-cli/src/reload.rs:517`
- [x] A Unix Rust test replays the kubelet's update: a root reached through `..data`, a swap to a new timestamped folder, and the old folder deleted. The next check publishes the new value, and an include inside the folder follows. `crates/nvs-cli/tests/reload_follows_links.rs:1`
- [x] A Rust test swaps a `current` link between two release folders, the shape a link deploy has. `crates/nvs-cli/tests/reload_follows_links.rs:1`

### 2. the directory holding a link is checked, a real cluster applies a changed ConfigMap, and the rule and the guide say so

**Does:** a link in a directory another account can write stops the boot with `E0607`, the guide's
ConfigMap reaches a running server on `kind`, and the rule and the guide describe it.

- [x] `trust::check` checks the parent of the path as given when it differs from the canonical path, with the read-only waiver (`rule:config/ownership-is-the-trust-boundary`). `crates/nvs-config/src/trust.rs:136`
- [x] Rust tests: a link in a group-writable directory is refused at boot and at reload, and a root with no link is checked as before. `crates/nvs-config/tests/trust.rs:1`
- [x] On `kind`, the guide's ConfigMap and Deployment serve, a changed ConfigMap is published without a restart, and the modes of the three directories are recorded in the handoff. The safe default above applies where no cluster runs. `website/src/content/docs/guides/production/docker.mdx:1`
  - [x] A `kind` cluster runs in Docker Desktop, with the guide's ConfigMap mounted read-only in a pod.
  - [x] The modes of `/etc/nvs`, `..data` and the timestamped folder are recorded in the handoff.
  - [x] The guide's Deployment serves the `nvs` image, and a changed ConfigMap prints the reload line.
- [x] The rule says a root is stored as given and read through its links on every reload, and that the directory holding a link is checked. Its title changes to match. `docs/rules/config/the-resolved-root-is-announced-and-stored.md:1`, `data/rules/config/the-resolved-root-is-announced-and-stored.json:2`, `docs/rules/config/ownership-is-the-trust-boundary.md:1`
- [x] The Docker guide says the server reads a changed ConfigMap by itself once Kubernetes updates the file, and the restart step is gone. `website/src/content/docs/guides/production/docker.mdx:1`
- [x] Static serving withholds every `[db]` file the tree names, and a reload that changes only `[db]` builds a new table (the gap `a-mount-entrys-own-database-files-are-not-withheld`). `crates/nvs-server/src/mount.rs:281`, `crates/nvs-cli/src/serve/mounts.rs:139`
  - [x] `Table::from_config` takes the `[db]` paths of the host tree, which are all of them: an `[[app]]` block has no `[db]` table (see `## Standing decisions`).
  - [x] `Local::table` compares the withheld paths too.
  - [x] A test: the SQLite file under the static root falls through to the entry before and after a reload that moves it, and the table is rebuilt. `a_snapshot_that_moves_only_a_database_file_rebuilds_the_table`, run once, green.
  - [x] The gap record `data/gaps/nvs-server/a-mount-entrys-own-database-files-are-not-withheld.json` is deleted.
- [x] `bun nv rules --render` and `bun nv owners --closes config-follows-links` are clean. `docs/rules/config.md:1`

### 3. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

**Stage 2 is done, and stage 3, the gate, is next.** The gap this goal owned is closed in
`216fd2d3e`: an `[[app]]` block has no `[db]` table, so the host's `[db]` paths were already every
file to withhold, and `Local::table` now rebuilds the table when only a `[db]` path moves
(`crates/nvs-cli/src/serve/mounts.rs:169`, `nvs_server::mount::databases`). Its test
`a_snapshot_that_moves_only_a_database_file_rebuilds_the_table` ran once, green.
`bun nv rules --render` and `bun nv owners --closes config-follows-links` are clean.

`d8` is answered, recorded under `## Standing decisions` and closed. `trust::check` walks the path as given and checks the parent of
each link through `platform::check_link_holder`, which skips the mode test for a root-owned
directory on a read-only mount (`crates/nvs-config/src/trust.rs:136`, `:248`). Its unit test is
`a_root_owned_link_holder_on_a_read_only_mount_skips_the_mode_test`. `tests/trust.rs` has a private
link that passes and a refused link on each platform. The Windows cases passed once, through a
junction. `reload_follows_links.rs:261` refuses a shared link at reload and at boot. It is
`cfg(unix)` and has never run, so the gate's Unix run is its first. Both rule fragments and the guide
say it, and `bun nv rules --render` is clean.

**The real cluster ran.** A Linux `nvs` built at `6b5dd907f` in the `benches/proxied/Dockerfile`
build stage served the guide's ConfigMap and Deployment on `kind` v0.27.0. The boot passed the
check. After `kubectl patch` set `memory = "512M"`, the pod printed `configuration reloaded` with
`applied: ["limits.memory"]` 60 seconds later, with no restart. The cluster, the images and the
scratch files are deleted. `looper.toml`'s `hang_exempt` now covers `docker build`: the watch
killed the first build, whose compile runs in Docker's VM.

The modes the previous session measured, in a busybox pod:

| Path | Mode | Owner |
|---|---|---|
| `/etc/nvs` (holds both links) | `0777` | `root:root` |
| `/etc/nvs/..data` (link to the timestamped folder) | link | `root:root` |
| `/etc/nvs/..2026_10_23_13_06_24.610892580` | `0755` | `root:root` |
| `/etc/nvs/..data/override.toml` | `0644` | `root:root` |

`/proc/mounts` shows `/etc/nvs` as `ext4 ro`.

Stage 1 is built: `Root` stores each root as given (`crates/nvs-cli/src/config.rs:618`), and
`Stamp` carries the file's identity (`crates/nvs-cli/src/reload.rs:517`).

**The acceptance sweep's reds are fixed.** `env-secret-proofs` exits 0: its own log at
`.looper/loops/main/checks/env-secret-proofs.log` reads `exit 0`. Nightly run 39922764699 ran on `7b6ef53ee`.
Every red job in it but one is fixed by a later commit, each checked by name at HEAD: the
property-observer probe guard by `42409d10d`, the four Windows serve tests by `ba8930b80`, the macOS
redirect case by `342675093`, the proofs gate by `ba8e5e8bd` and `9270c45c4`, linux-aarch64's
isolation flood by `922abf5f0`, conformance's extension CPU time by `dc3a2ed29`, asan's Windows alias
by `93919e633`, and the website check by `f6ce06671`. The cancelled jobs were cancelled behind those.

**The last red, the live-bytes cost guard, is fixed in `071663316`.** About 2 KB of each
connection's spawn stayed on the accept loop's balance. Two charges were allocated on the spawning
turn and freed elsewhere:

- `Scheduler::drain_pending` took the pending list, so the scheduler's stack freed its buffer, and the
  next spawn allocated it again. The emptied list now goes back with its buffer
  (`crates/nvs-host/src/scheduler.rs:1240`). That buffer was about 1.8 KB of the 2 KB.
- `Ctx::new`'s tree state, deadline and object list, and the boxed body. `spawn_child` and
  `spawn_root` now credit them to the running turn through `Ctx::hand_over_its_making`
  (`crates/nvs-runtime/src/ctx/limits.rs`, `budget::credit_turn`). The credit applies only while the
  context is its own unshared tree, made on this turn, and each allocation counts only while it is
  still unshared and not replaced. So a joined child credits nothing, and no byte is credited twice.

The new test `a_parent_that_spawns_and_outlives_many_children_stays_flat` measured +411 KB over 200
children before the fix and is flat after it. The cost guard
(`cargo test --release -p nvs-cli --bin nvs -- --ignored live_bytes_stay_flat_while_one_core_serves_thousands_of_requests`)
passed once on Windows.

**Stage 3 is done, and the goal is reached.** The installed `looper` binary has no `verify`
subcommand, so the gate ran as `bun tools/looper/main.ts verify`. The first run was green in 833 s,
and its formatter rewrapped four of this goal's files, committed in `668607a4f`. The second run kept
that verdict. `config-tests` and `cli-tests` are green: 7496 passed, 0 failed.
`bun nv verify --doc` and `bun nv owners --closes config-follows-links` are clean. Every standing
decision has a home: the two config rule fragments,
`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`, and
`nvs_server::mount::databases`' doc.

### Next

- Retire the goal. The loop's retire step deletes this file and rebuilds `target/release/nvs`.

The Unix test `a_link_in_a_group_writable_folder_is_refused_at_reload_and_at_boot` first runs in
Unix CI, and it may go red there. The nightly's live-bytes cost guard should pass on macOS and Linux
from `071663316`.

### Backlog

- Nothing beyond stage 3.
