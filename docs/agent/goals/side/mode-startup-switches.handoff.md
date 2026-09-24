# Handoff

## State

**Side goal `mode-startup-switches` — an unwritten `dispatch` and `static` follow the mode a
configuration names — has just started; nothing of it has landed yet.** It runs in its own worktree
under `loop.py --side mode-startup-switches`, over main's carried floor, and lands on `main` when it
is green.

The design is the user's and the rule's. The goal file's § *Standing decisions* answers every call a
session meets, and no session re-decides one. § *What is on disk today, measured* is the starting
point.

## Next group

**Stage 2: the switches follow the mode** — one file set: `crates/nvs-config/src/server.rs`,
`crates/nvs-server/src/mount.rs`, `crates/nvs-cli/src/serve/mounts.rs`.

- [ ] **One resolving function in `nvs-config`**, reading the mode as `Revalidation::from_config`
      does (`crates/nvs-config/src/cache.rs:517`).
- [ ] **`Table::from_config` calls it** (`crates/nvs-server/src/mount.rs:176`), and § *Decision*
      (`mount.rs:35`) is rewritten whole.
- [ ] **`switches` compares its result** (`crates/nvs-cli/src/serve/mounts.rs:125`).
- [ ] **The four named tests** in the goal's `.toml`.

## Backlog

- **Stage 3: the template, the server chapter and the rule say it**, and `dossier.py --id` names
  any proof still owed.
- When the last check goes green the side run lands itself on `main`.
