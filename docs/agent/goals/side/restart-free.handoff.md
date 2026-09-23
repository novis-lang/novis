# Handoff

## State

**Side goal `restart-free` — a running server takes every code change without a restart, and every
config change it can — has just started; nothing of it has landed yet.** It runs in its own worktree
under `loop.py --side restart-free`, over main's carried floor, and lands on `main` when it is green.

The design is settled and is the user's. The goal file's § *Standing decisions* answers every call
a session meets, and no session re-decides one. § *What is on disk today, measured* is what a probe
program showed against the release binary before the goal was written; each stage confirms its part
before changing it.

## Next group

**Stage 2: every file a program reached is watched** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-config/src/cache.rs`, `crates/nvs-config/src/default.toml`.

- [ ] **The decision record for source revalidation** — written first, from the goal file's Stages 2
      to 4 and § *Standing decisions*, at the next free number on `main`. It `modifies`
      `config/an-edit-reaches-the-next-request-without-a-restart`,
      `config/opcache-revalidation-is-system-class` and `config/a-startup-default-is-never-flipped`,
      and those fragments are rewritten whole with it.
- [ ] **The `live_edit` harness** — `crates/nvs-cli/tests/live_edit.rs`: start the built `nvs serve`
      on a free port over a program in a temporary directory, edit it, poll until the answer changes
      or a bound expires. Every Stage 2 to 4 test uses it.
- [ ] **The unit is keyed on the whole program** — `crates/nvs-cli/src/script.rs:@Compiler`: a
      compile records every file it read, and a check of a unit checks all of them.
- [ ] **A discovery query's directories join the check** — wherever `Core\Program::implementing`
      lists directories (`python tools/peek.py --locate implementing`).
- [ ] **`validate = "never"` is removed** — `crates/nvs-config/src/cache.rs:@Validate`, the mode row,
      and the template's `[opcache]` block.
- [ ] **The Stage 2 tests** the goal's `.toml` names.

## Backlog

- **Stage 3: off the request path, settle, atomic link switch, bounded memory** — same files, plus
  `crates/nvs-cli/src/serve.rs` and `crates/nvs-config/src/directive.rs`.
- **Stage 4: the mount table follows the disk** — `crates/nvs-cli/src/serve.rs:@table_for`,
  `crates/nvs-config/src/mount.rs`.
- **Stage 5: every reloadable key reloads, and the census** — its own decision record first.
- **Stage 6: the configuration applies itself; `Boot` is three keys.**
- **Stage 7: the template, the reference section, the feature proofs.**
- When the last check goes green the side run lands itself on `main`.
