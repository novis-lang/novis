# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2 and 3 are complete.** A request for a path with a pointer does one map lookup and makes
no file-system call (`Compiler::compiled`). `Compiler::revalidate` runs on one `nvs-revalidate`
thread (`script::watch`), once per `revalidate_freq`, from `nvs serve` and `nvs run`. A change is
held back until the program has been quiet for `settle`. A compile reads the program through its
entry's real path, and a switched link discards it. The unit table holds at most two units per path
after ten thousand edits (`units_held_stay_bounded_after_ten_thousand_edits`, a unit test in
`script.rs`: `nvs-cli` has no library, so `live_edit` cannot read `Compiler::held`). That test was
moved out of the `live_edit` check into a check of its own in the goal's toml. It takes about 150s
in a debug build, one compile per edit, and so it is the tail of the `nvs` unit-test binary.

Calls in ADR 0218 that are mine and not confirmed with the user: a mount re-expansion that meets a
match boot would refuse logs it and leaves it out (§ 9), keeping the replaced unit (§ 7, § 8), and
the `FLOOR` of 10ms between two watcher passes.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 4: the mount table follows the disk** — one file set: `crates/nvs-cli/src/serve.rs`,
`crates/nvs-config/src/mount.rs`, `crates/nvs-cli/src/script.rs`, `crates/nvs-cli/tests/live_edit.rs`.

- [ ] **A scan is expanded again in the background** (ADR 0218 § 9,
      `rule:http-server/a-mount-table-expands-at-boot`, which this modifies). Each scanned
      directory's stamp is compared on the watcher's pass, and one that moved is listed again with
      `nvs_config::mount::expand` at `crates/nvs-config/src/mount.rs:284`. The boot expansion is
      `crates/nvs-cli/src/serve.rs:1830`, and the watcher pass is `Compiler::revalidate` at
      `crates/nvs-cli/src/script.rs:658`. A new module is compiled, has the `origin` check run on it
      and is served; a match boot would refuse is logged and left out. Tests
      `a_new_module_under_a_mount_scan_is_served_without_a_restart` and
      `a_removed_module_under_a_mount_scan_answers_404` in `crates/nvs-cli/tests/live_edit.rs:1018`'s
      file.
- [ ] **A new scanned module that does not compile fails only its own requests**, and **an explicit
      mount entry that appears is served** (ADR 0218 § 9). Tests
      `a_new_scanned_module_that_does_not_compile_fails_only_its_own_requests` and
      `an_explicit_mount_entry_that_appears_is_served_without_a_restart`, same file, over the same
      expansion at `crates/nvs-cli/src/serve.rs:1830`. Then shrink the **What is on disk** paragraph
      of `rule:config/an-edit-reaches-the-next-request-without-a-restart`.

## Backlog

- Stage 5, every reloadable key really reloads, and the census — `docs/agent/goals/side/restart-free.md` § Stage 5.
- Stage 6, the configuration applies itself — same file, § Stage 6, and a second decision record.
- The ten-thousand-edit test costs one debug compile per edit; a cheaper compile path would shorten it — `crates/nvs-cli/src/script.rs`.
