# Handoff

## State

**Side goal `restart-free` — a running server takes every code change without a restart, and every
config change it can.** Stage 2's decision record is on disk: ADR 0218 (source revalidation, Stages 2
to 4), with the five rules it modifies rewritten to the decided design —
`rule:config/an-edit-reaches-the-next-request-without-a-restart`,
`rule:config/opcache-revalidation-is-system-class`, `rule:config/a-startup-default-is-never-flipped`,
`rule:packaging/autoload-probes-fold-into-the-cache-key` and `rule:http-server/a-mount-table-expands-at-boot`.
Each fragment ends in a **What is on disk** paragraph saying what the code does not do yet; the
session that lands a behaviour deletes or shrinks that paragraph. No code has changed yet.

Two calls in ADR 0218 are this session's, not the user's, and are not confirmed: the table keeps, per
path, the unit in force **and the one it replaced**, so a reverted edit is a pointer swap (§ 7, § 8);
and a mount re-expansion that meets a match boot would refuse logs it and leaves it out (§ 9).
`settle` took `validate`'s startup row, so the startup table is `dispatch`, `static`, `settle`.

The ADR's § *Diagnostics* names no code for `validate = "never"`: the slice that removes it uses
whatever the config crate reports for a value outside a directive's set, and adds that refusal if
there is none (`crates/nvs-config/src/cache.rs` module doc says neither directive is refused at boot).

`verify.py` is green through clippy in this worktree; its `extension` leg fails on `tsc` not found,
the playbook's missing-`editors/vscode/node_modules` trap, which nothing here touches.

## Next group

**Stage 2: every file a program reached is watched** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-config/src/cache.rs`, `crates/nvs-config/src/default.toml`.

- [ ] **`validate = "never"` is removed** — `crates/nvs-config/src/cache.rs:382` (`Validate`, its
      `Never` arm, `started_in` at :411, `of` at :421, `validate_of`'s `Bool(false)` at :527), the
      step-1 `Never` test at `crates/nvs-cli/src/script.rs:530`, and the template's `[opcache]` block at
      `crates/nvs-config/src/default.toml:854`. Tests `validate_never_does_not_load_and_names_mtime_and_hash`
      and `validate_defaults_to_mtime_in_production_and_development` (`rule:config/opcache-revalidation-is-system-class`).
      Shrink that rule's and `rule:config/a-startup-default-is-never-flipped`'s *What is on disk* paragraphs.
- [ ] **The `live_edit` harness** — `crates/nvs-cli/tests/live_edit.rs`: start the built `nvs serve`
      on a free port over a program in a temporary directory, edit it, poll until the answer changes
      or a bound expires. Every Stage 2 to 4 test uses it. `crates/nvs-cli/tests/spawn_entry.rs:32`
      is the shape an existing suite uses to run the built binary (`env!("CARGO_BIN_EXE_nvs")`).
- [ ] **The unit is keyed on the whole program** — `crates/nvs-cli/src/script.rs:341` (`Compiler`),
      its `key` at :668 and `compiled` at :515: a compile records every file it read, and a check of a
      unit checks all of them (`rule:config/an-edit-reaches-the-next-request-without-a-restart`,
      ADR 0218 § 2). `crates/nvs-cli/src/cache.rs:1492` (`program_digest`) is the digest to key on.
- [ ] **A discovery query's directories join the check** — wherever `Core\Program::implementing`
      lists directories — `crates/nvs-hir/src/autoload.rs:631` is the resolver's one `read_dir`,
      not yet confirmed to be that scan —
      `rule:packaging/autoload-probes-fold-into-the-cache-key`.

## Backlog

- **The rest of the Stage 2 tests** the goal's `.toml` names, after the four items above.
- **Stage 3: off the request path, settle, atomic link switch, bounded memory** — ADR 0218 § 3 to § 8;
  same files plus `crates/nvs-cli/src/serve.rs` and `crates/nvs-config/src/directive.rs`.
- **Stage 4: the mount table follows the disk** — ADR 0218 § 9; `crates/nvs-cli/src/serve.rs:@table_for`,
  `crates/nvs-config/src/mount.rs`.
- **Stage 5: every reloadable key reloads, and the census** — the second decision record first.
- **Stage 6: the configuration applies itself; `Boot` is three keys.**
- **Stage 7: the template, the reference section, the feature proofs.**
