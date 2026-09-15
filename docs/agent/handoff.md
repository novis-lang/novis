# Handoff

## State

**Goal `m8-db-queue`, stage 9 is complete: the socket leg and the CI leg both land**, and all three
of that stage's checks pass locally — `python tools/db-matrix.py --all` prints the five TCP legs and
then `mysql`, `mariadb` and `postgres` `over a socket: ok`.

`tests/db/compose.yaml` bind-mounts each of those three servers' *own* default socket directory onto
the host under `${NOVIS_DB_SOCKET_DIR:-/mnt/host/wsl/novis-db}/<service>`, and the `certs` service
makes each one writable by the uid 999 all three images drop to. Nothing moved a server off its
default socket path, so every image's own healthcheck still reaches it. On Windows the leg's `cargo
test` runs inside WSL over `/var/tmp/nvs-target-wsl`, because a Windows build has no `AF_UNIX`
transport and the socket a Linux container publishes is the distro's to reach;
`tools/db-matrix.py` § *The socket leg* is the home for all of that, and `SOCKET_SUITES` for why the
leg runs `-p nvs-db` alone.

`crates/nvs-db/src/matrix.rs` gap 1 is retired. `.github/workflows/ci.yml` gained a `database` job
running `python tools/db-matrix.py --all` on `ubuntu-latest`, gated by a new `db` lane in
`tools/ci-changes.py`; CI itself is still not running, so that leg is proved by reading those two
files, as the goal's standing decisions say.

## Next group

**Stage 10: the rulebook and the module docs** — one file set: `docs/rules/core-classes.json`,
`crates/nvs-stdlib/src/db/mod.rs` and `crates/nvs-types/src/derive.rs`. This is the last stage of the
goal; the owner gates are met once no `— owner: m8-db-queue` tag is left, and four are.

- [ ] **Flip stage 2's three rules from `designed` to `shipped`**, with `guardedBy` filled from this
      goal's own cases and tests, then `python tools/rules.py --render`:
      `docs/rules/core-classes.json:487` (`rule:core-classes/a-stream-parks-its-read-on-the-connection`),
      `docs/rules/core-classes.json:405` (`rule:core-classes/server-version-is-what-the-server-said`)
      and `docs/rules/core-classes.json:549` (`rule:core-classes/a-unique-key-reads-nulls-as-distinct`).
- [ ] **Rewrite `crates/nvs-stdlib/src/db/mod.rs:212`'s `# Known gaps` as a whole** — gaps 3–4 are
      built, and their owner tags are `crates/nvs-stdlib/src/db/mod.rs:291` and `:308`. Gaps 1–2 are
      `unowned` and stay, so the section shrinks rather than going.
      `rule:core-classes/db-streaming` and `rule:core-classes/db-one-api` are what it now describes.
- [ ] **Rewrite `crates/nvs-types/src/derive.rs:42`'s `# Known gaps` the same way** — gaps 1–2 are
      built, tags at `crates/nvs-types/src/derive.rs:76` and `:98`; gap 3 is
      `rule:core-classes/derive-attribute`'s and `unowned`, so it stays and is renumbered.

## Backlog

- `crates/nvs-stdlib/src/queue.rs` gaps 1–2 and `crates/nvs-stdlib/src/db/mod.rs` gaps 1–2 are goal
  `unowned-closures`'s — `docs/agent/loop-goal.md` § *Standing decisions*, *Not this goal*.
- The socket leg asserts `crates/nvs-db`'s case list alone: `queue`, `db_stream` and the worker's
  case gate themselves out on a `Location::Socket`. `tools/db-matrix.py`'s `SOCKET_SUITES` says what
  a case needing a socket would change.
- A Linux host runs the socket leg natively and has never been tried; `NOVIS_DB_SOCKET_DIR` is the
  one field that has to name the same directory at both ends there.
