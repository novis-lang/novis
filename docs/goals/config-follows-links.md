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
    status: open
    context: [crates/nvs-server/src/mount.rs, crates/nvs-cli/src/serve/mounts.rs, crates/nvs-config/src/snapshot.rs, website/src/content/docs/guides/production/docker.mdx]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
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

**The gap this goal owns is closed here, by withholding every `[[app]]` block's `[db]` files.**
Security-hardening handed `a-mount-entrys-own-database-files-are-not-withheld` to this goal. An
entry's snapshot is folded lazily (`Published::entry`), and the mount table is chosen before the
entry is known. So the table withholds the `[db]` paths of the host tree and of every `[[app]]`
block, matched or not. Withholding a file one entry does not use only refuses a static request for a
database file, so it costs no correct request. Memory: four paths per `[db]` block, per table.

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
- [ ] Static serving withholds every `[db]` file the tree names, an `[[app]]` block's included, and a reload that changes only `[db]` builds a new table (the gap `a-mount-entrys-own-database-files-are-not-withheld`). `crates/nvs-server/src/mount.rs:281`, `crates/nvs-cli/src/serve/mounts.rs:139`
  - [ ] `Table::from_config` takes the `[db]` paths of the host tree and of every `[[app]]` block.
  - [ ] `switches` in `serve/mounts.rs` compares the withheld paths too.
  - [ ] A served test fetches an `[[app]]` block's SQLite file under the static root and gets `404`, before and after a reload that moves it.
  - [ ] The gap record `data/gaps/nvs-server/a-mount-entrys-own-database-files-are-not-withheld.json` is deleted.
- [ ] `bun nv rules --render` and `bun nv owners --closes config-follows-links` are clean. `docs/rules/config.md:1`

### 3. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

**Stage 2 is built except for one item: the gap this goal owns.** `d8` is answered, recorded under
`## Standing decisions` and closed. `trust::check` walks the path as given and checks the parent of
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

The acceptance sweep's `env-secret-proofs` exits 0 on disk, and run 39922764699 is the nightly the
previous handoff traced to fix commits. Clippy is clean. The only tests run were `--test trust`,
once, green on Windows.

### Next

**Stage 2's last item: the gap `a-mount-entrys-own-database-files-are-not-withheld`**, whose design
is under `## Standing decisions`. `bun nv owners --closes config-follows-links` names it, and it is
the only thing it names.

- [ ] `Table::from_config` withholds the `[db]` paths of every `[[app]]` block beside the host's. `AppBlocks` (`crates/nvs-config/src/snapshot.rs:416`) has the blocks, and the table is built at `crates/nvs-server/src/mount.rs:281`. A block's `path` is absolute once resolved: confirm that for an `[[app]]` block before you filter on `is_absolute`.
- [ ] `switches` compares the withheld paths, so a reload that changes only `[db]` builds a new table. Today it keeps the old one. `crates/nvs-cli/src/serve/mounts.rs:139`
- [ ] A test: an `[[app]]` block's SQLite file under the static root is `404`, before and after a reload. The host-tree cases to copy are unit tests in `crates/nvs-server/src/mount.rs:1832` and `crates/nvs-server/src/mount.rs:1878`.
- [ ] Delete the gap record, then `bun nv owners --closes config-follows-links` is clean. Set stage 2 to `done`. `data/gaps/nvs-server/a-mount-entrys-own-database-files-are-not-withheld.json:1`

Tests that may go red: none expected. `live_config.rs` keeps its roots without links. The new Unix
test `a_link_in_a_group_writable_folder_is_refused_at_reload_and_at_boot` is unrun.

### Backlog

- Stage 3, the gate.
