# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Stages 2 to 7 are complete on disk, and nothing in the goal's
own list is open.

**The goal is held red by a second floor check it cannot fix, so this session wrote `BLOCKED`.**
The re-pointed `nvs-config (the validate default)` check is green now. The next red one is
`nvs-config (four rows, System throughout, split Boot and Reload)`, which names two tests commit
`d968c8079` renamed when the whole of `[queue]` became `Reload` under the standing decision that
only three keys restart. A side run may not edit `docs/agent/loop-goal.toml`.

A static scan of every `cargo-named` name in `docs/agent/loop-goal.toml` against the tree's `fn`
declarations finds no other name this branch broke: the two further misses it lists,
`two_body_spellings_in_one_spec_are_refused` and
`webcrypto_jwe_vectors_reencrypt_to_the_same_token_from_the_same_randomness`, are missing on `main`
too. The floor's program and command checks after the red one have not run on this branch yet.

The worktree needs two git-ignored things the main checkout holds: `tests/db/ca.crt`, copied in, and
`editors/vscode/node_modules`, installed with `npm ci` for the `vscode (headless)` floor check. Both
are in the playbook, and both are in place now.

Calls that are mine and not confirmed: the two Stage 7 checks re-scoped in
`docs/agent/goals/side/restart-free.toml`, the mark spelled `# default; restart required`, and the
reference saying a `[[server.mount]]` table resolves its links at boot.

## Next group

**Stage 7: the `[queue]` floor check the user re-points** — one file set: `docs/agent/loop-goal.toml`
and `docs/agent/goals/dossier/120-core-io-file-and-1-more.toml`.

- [ ] **The user re-points the floor check** `nvs-config (four rows, System throughout, split Boot and
      Reload)` at `docs/agent/loop-goal.toml:7156` on this branch, as they did for the validate
      default: `connection_and_workers_are_boot_and_max_attempts_and_visibility_are_reload` becomes
      `every_queue_key_reloads` (`crates/nvs-config/tests/directives.rs:2040`), and
      `a_reload_that_changes_workers_carries_the_running_value_and_names_the_key` becomes
      `a_reload_that_changes_workers_publishes_the_new_count`
      (`crates/nvs-config/tests/snapshot.rs:392`). The check's name also says "split Boot and
      Reload", which is no longer true of `[queue]`. `rule:config/reloadability-is-its-own-field`.

## Backlog

- The dossier goal `120-core-io-file-and-1-more` on `main` still names both old queue tests and
  `the_validate_default_is_selected_by_the_run_mode`; its copy needs the same re-points before it
  goes live (`docs/agent/goals/README.md` § *Side goals*).
