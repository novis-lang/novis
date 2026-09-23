# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Both are on disk, and 0219 is the last record number this goal
takes. Stages 2 to 7 are complete.

**Stage 7 is done.** The template marks the three restart keys `; restart required`, a test holds it
to `DIRECTIVES`, and `docs/reference/tools/25-server.md` § *What reaches a running server* is the
roster feature `tools:server/what-reaches-a-running-server`. The `[opcache]` example, its `about.md`
and its attack now describe `mtime` in both modes and `settle`, and the attack also pushes
`opcache.settle`. The Stage 7 `--comments` check prints `each inside the bounds` over its six
directories. `verify.py --doc` is green after a broken `Self::check` link in
`crates/nvs-cli/src/script.rs` was pointed at `look` and `take`.

**The goal is held red by one floor check it cannot fix, so this session wrote `BLOCKED`.** Main's
carried floor check `nvs-config (the validate default)` names
`the_validate_default_is_selected_by_the_run_mode`. This goal renamed that test to
`validate_defaults_to_mtime_in_production_and_development` in `crates/nvs-config/tests/snapshot.rs`,
because the standing decision removes `never`. A side run may not edit main's `loop-goal.toml`.

Calls that are mine and not confirmed: the two Stage 7 checks re-scoped in
`docs/agent/goals/side/restart-free.toml` (`--gate --only` the section, `--comments` over six
directories), the mark spelled `# default; restart required`, and the reference saying a
`[[server.mount]]` table resolves its links at boot.

`verify.py`'s `extension` leg fails on `tsc` not found (no `editors/vscode/node_modules`), which
nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main checkout.

## Next group

**Stage 7: the floor check the user re-points** — one file set: main's `docs/agent/loop-goal.toml`
and `docs/agent/goals/dossier/115-core-http-response-and-1-more.toml`.

- [ ] **The user re-points the floor check** `nvs-config (the validate default)` at
      `docs/agent/loop-goal.toml:3862` on `main`, from `the_validate_default_is_selected_by_the_run_mode`
      to `validate_defaults_to_mtime_in_production_and_development`
      (`crates/nvs-config/tests/snapshot.rs:766`), in both toml copies.
      `rule:config/opcache-revalidation-is-system-class`. Nothing else in the goal is open.

## Backlog

- A `current` link switch does not reach a server with a `[[server.mount]]` table until it restarts
  — owner: `rule:config/an-edit-reaches-the-next-request-without-a-restart` § *What is on disk*;
  ask the user whether this goal must close it.
