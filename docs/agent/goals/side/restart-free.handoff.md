# Handoff

## State

**Side goal `restart-free` — a running server takes every code change without a restart, and every
config change it can.** ADR 0218 (source revalidation, Stages 2 to 4) is on disk with the five rules it
modifies; each fragment ends in a **What is on disk** paragraph the landing session shrinks.

`validate = "never"` is gone (`Validate` is `Mtime | Hash`, `mtime` the default in both modes, `E0601`
at load for anything else). The check still runs inside the resolve, on the request path, and the key's
digest is still the entry file's content alone: probed by hand against `target/debug/nvs.exe serve`, an
edit to a `require`d file is not answered until the entry file changes.

The `live_edit` harness is on disk: `crates/nvs-cli/tests/live_edit.rs`, whose module doc says how
`Server::start`, `get`, `awaits`, `awaits_body` and `write` work. Two of the check's eleven tests are
written and pass against today's code (the entry-file edit, and a broken entry file then its fix); the
other nine are not written, because each fails until the item it tests lands.

Two calls in ADR 0218 are not the user's and are not confirmed: the table keeps, per path, the unit in
force and the one it replaced (§ 7, § 8); a mount re-expansion that meets a match boot would refuse
logs it and leaves it out (§ 9). The startup table is `dispatch`, `static`, `settle`; `settle` does not
exist yet.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`), which
nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main checkout.

## Next group

**Stage 2: every file a program reached is watched** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/tests/live_edit.rs`, `crates/nvs-cli/src/cache.rs`.

- [ ] **The unit is keyed on the whole program** — `crates/nvs-cli/src/script.rs:341` (`Compiler`),
      its `key` near :650 and `compiled` near :500: a compile records every file it read, and a check
      of a unit checks all of them (`rule:config/an-edit-reaches-the-next-request-without-a-restart`,
      ADR 0218 § 2). `crates/nvs-cli/src/cache.rs:1492` (`program_digest`) is the digest to key on.
      Lands with `an_edit_to_a_required_file_reaches_the_next_request`,
      `an_edit_to_an_autoloaded_class_reaches_the_next_request` and
      `a_deleted_required_file_fails_the_requests_that_reach_it` in
      `crates/nvs-cli/tests/live_edit.rs:250`, on the harness's `Server`.
- [ ] **A probe miss that a new file fills recompiles** — the two autoload cases,
      `a_new_class_file_under_an_autoload_root_is_found_without_a_restart` and
      `a_file_that_shadows_an_autoload_probe_miss_takes_over_without_a_restart`, in
      `crates/nvs-cli/tests/live_edit.rs:250`; the probe trace is already keyed
      (`rule:packaging/autoload-probes-fold-into-the-cache-key`), so these may pass once the first item does.
- [ ] **A discovery query's directories join the check** — wherever `Core\Program::implementing`
      lists directories — `crates/nvs-hir/src/autoload.rs:631` is the resolver's one `read_dir`,
      not yet confirmed to be that scan —
      `rule:packaging/autoload-probes-fold-into-the-cache-key`.

## Backlog

- **The remaining Stage 2 tests** — the reverted edit, the queue job and scheduled fire, the running
  WebSocket; the goal's `.toml` names them.
- **`revalidate_freq` refused at load** when it is not a duration — `crates/nvs-config/src/cache.rs`
  module doc records it as not refused yet.
- **Stage 3: off the request path, settle, atomic link switch, bounded memory** — ADR 0218 § 3 to § 8;
  same files plus `crates/nvs-cli/src/serve.rs` and `crates/nvs-config/src/directive.rs`.
- **Stage 4: the mount table follows the disk** — ADR 0218 § 9; `crates/nvs-cli/src/serve.rs:@table_for`,
  `crates/nvs-config/src/mount.rs`.
- **Stage 5: every reloadable key reloads, and the census** — the second decision record first.
- **Stage 6 and 7** — the configuration applies itself; the template, the reference, the feature proofs.
