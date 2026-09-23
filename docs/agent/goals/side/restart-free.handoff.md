# Handoff

## State

**Side goal `restart-free` — a running server takes every code change without a restart, and every
config change it can.** ADR 0218 (source revalidation, Stages 2 to 4) is on disk with the five rules it
modifies; each fragment ends in a **What is on disk** paragraph the landing session shrinks.

`validate = "never"` is gone: `Validate` is `Mtime | Hash`, `mtime` is the default in both modes, and
`nvs_config::cache::validate` refuses `never`, `false` and any other word at load with `E0601`, naming
`mtime` and `hash` (called from `crates/nvs-config/src/resolve.rs` after `log::validate`). `true` still
reads as `mtime`. `revalidate_freq` is still not refused at load. The check still runs inside the
resolve, on the request path, and the key's digest is still the entry file's content alone.

Two calls in ADR 0218 are not the user's and are not confirmed: the table keeps, per path, the unit in
force and the one it replaced (§ 7, § 8); a mount re-expansion that meets a match boot would refuse
logs it and leaves it out (§ 9). The startup table is `dispatch`, `static`, `settle`; `settle` does not
exist yet.

The floor check `nvs queue migrate creates both tables` failed here on a missing `tests/db/ca.crt`,
a git-ignored fixture; it was copied in from the main checkout (playbook, *Running things*).
`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`), which
nothing here touches.

## Next group

**Stage 2: every file a program reached is watched** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/tests/live_edit.rs`, `crates/nvs-cli/src/cache.rs`.

- [ ] **The `live_edit` harness** — `crates/nvs-cli/tests/live_edit.rs`: start the built `nvs serve`
      on a free port over a program in a temporary directory, edit it, poll until the answer changes
      or a bound expires. Every Stage 2 to 4 test uses it. `crates/nvs-cli/tests/spawn_entry.rs:32`
      is the shape an existing suite uses to run the built binary (`env!("CARGO_BIN_EXE_nvs")`).
      `rule:config/an-edit-reaches-the-next-request-without-a-restart`.
- [ ] **The unit is keyed on the whole program** — `crates/nvs-cli/src/script.rs:341` (`Compiler`),
      its `key` near :650 and `compiled` near :500: a compile records every file it read, and a check
      of a unit checks all of them (`rule:config/an-edit-reaches-the-next-request-without-a-restart`,
      ADR 0218 § 2). `crates/nvs-cli/src/cache.rs:1492` (`program_digest`) is the digest to key on.
- [ ] **A discovery query's directories join the check** — wherever `Core\Program::implementing`
      lists directories — `crates/nvs-hir/src/autoload.rs:631` is the resolver's one `read_dir`,
      not yet confirmed to be that scan —
      `rule:packaging/autoload-probes-fold-into-the-cache-key`.

## Backlog

- **The rest of the Stage 2 tests** the goal's `.toml` names, after the three items above.
- **`revalidate_freq` refused at load** when it is not a duration — `crates/nvs-config/src/cache.rs`
  module doc records it as not refused yet.
- **Stage 3: off the request path, settle, atomic link switch, bounded memory** — ADR 0218 § 3 to § 8;
  same files plus `crates/nvs-cli/src/serve.rs` and `crates/nvs-config/src/directive.rs`.
- **Stage 4: the mount table follows the disk** — ADR 0218 § 9; `crates/nvs-cli/src/serve.rs:@table_for`,
  `crates/nvs-config/src/mount.rs`.
- **Stage 5: every reloadable key reloads, and the census** — the second decision record first.
- **Stage 6 and 7** — the configuration applies itself; the template, the reference, the feature proofs.
