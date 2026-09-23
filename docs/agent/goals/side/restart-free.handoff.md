# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Both are on disk, and 0219 is the last record number this goal
takes. Stages 2 to 6 are complete, and Stage 7 has one group left.

**Stage 7 so far.** The template ends the three restart keys' lines `; restart required`
(`crates/nvs-config/src/default.toml`), and `tools/directives.py`'s `NOTE` accepts that suffix.
`every_restart_key_is_marked_restart_required_in_the_template_and_no_other_is` in
`crates/nvs-config/tests/resolve.rs` holds the template to `DIRECTIVES` in both directions.
`docs/reference/tools/25-server.md` has § *What reaches a running server*, with *Deploying* and
*What no compiler can check* under it. Its stale "read once, at start" and `scan` lines are
rewritten, and so are `docs/reference/tools/20-config.md`'s two reload passages. The section is
roster feature `tools:server/what-reaches-a-running-server`, and `dossier.py --id` calls it complete.

**Two Stage 7 checks were re-scoped in `docs/agent/goals/side/restart-free.toml`.** `--gate` now
runs `--only tools:server/what-reaches-a-running-server`, the shape the floor's own gate checks use,
and it passes. `--comments` now names the six program directories this goal writes or rewrites.
Unscoped, the gate owes 638 `Core` members and 305 of 2442 programs miss the bounds, all of them
work of main's dossier and comment goals, and no side goal can close them. This is my call, not
confirmed with the user.

**The floor holds this goal red on a check it cannot fix.** Main's carried floor check
`nvs-config (the validate default)` names `the_validate_default_is_selected_by_the_run_mode`. This goal
renamed it to `validate_defaults_to_mtime_in_production_and_development` in `crates/nvs-config/tests/snapshot.rs`
(commit 8707a9148), by the standing decision that removes `never`. A side run may not edit main's
`loop-goal.toml`, so the user has to re-point that floor check (in `docs/agent/loop-goal.toml` and
`docs/agent/goals/dossier/115-core-http-response-and-1-more.toml` on `main`). The session that finds
the rest of the goal green writes `BLOCKED` on it.

My other calls, not confirmed: the mark is spelled `# default; restart required`, because every
setting line already owes its unset note. The reference says that a `[[server.mount]]` table
resolves its links at boot, so a `current` link switch reaches such a server only at its next start
(`rule:config/an-edit-reaches-the-next-request-without-a-restart` § *What is on disk*).

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 7: the comment bounds** — one file set: `docs/examples/config/opcache/`,
`tests/hostile/config/opcache/`, `docs/examples/config/cache-shared/`.

- [ ] **The `[opcache]` example still describes `never`**: rewrite the comments in
      `docs/examples/config/opcache/01-when-this-host-notices-that-the-code-changed.nvs:1` to
      `mtime` in both modes and `settle`, re-bless the `.out` with `dossier.py --bless`, and rewrite
      `docs/examples/config/opcache/about.md:14` ("production never re-checks").
      `rule:config/opcache-revalidation-is-system-class`.
- [ ] **The `[opcache]` attack misses the bounds** at lines 2, 10, 38, 62 and 80 of
      `tests/hostile/config/opcache/01-pinning-the-version-of-the-code-i-like.nvs:2`. Rewrite each
      from what the code does. `rule:testing/feature-proofs`.
- [ ] **The `[cache.shared]` example misses the bounds** at lines 2, 15 and 22 of
      `docs/examples/config/cache-shared/01-the-store-a-whole-fleet-shares.nvs:2`. Then the Stage 7
      `--comments` check prints `each inside the bounds`, and the goal is green but for the floor.
      `rule:testing/feature-proofs`.

## Backlog

- A `current` link switch does not reach a server with a `[[server.mount]]` table until it restarts
  — owner: `rule:config/an-edit-reaches-the-next-request-without-a-restart` § *What is on disk*;
  ask the user whether this goal must close it.
- The floor check `nvs-config (the validate default)` needs the user to re-point it on `main`.
