# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, Stage 4's records are valid, and Stage 8's
checks are green. Stage 8's prose still owes the copy-free goal switch and the pack measured from
`nv orient`.** `bun nv check` prints `nv check: 0 findings`, and `bun nv render --check` is current.
The driver's red check is Stage 9's `bun nv audit goals`, which is the cutover and is not started.

`bun nv session --wrap` validates and applies a wrap by itself, and no longer calls Python to retire
bullets. `tools/nv/cmd/playbook.ts` is `tools/playbook.py`'s `expiry_report`, `holds`, `retire` and
`prune_manifests`, ported. It also answers `bun nv playbook --show <selector>` and `--retire
[--dry-run]`. A retired bullet deletes its fragment file and its record. The selectors that named only
that bullet are dropped from every goal `.toml`, from `docs/agent/loop-goal.toml` and from every live
goal record. Over the tree the TypeScript and Python expiry reports are identical on three dates, and a
real wrap in a scratch worktree retired a planted bullet and committed all five paths.
`recordPack` still measures `python tools/orient.py`, because the running driver still pipes that pack.

Every playbook record has the same `lead`, `body` and `until` as its fragment file. Eight differ in
`files` alone, where the record is right. `nv import --write` still rewrites those eight: the bullet
`docs/agent/playbook/tooling/bun-nv-import-write-rewrites-a-data-playbook-bullet-back-to.md` says
what to do.

`tools/orient.py`, `chain.py`, `session.py`, `playbook.py` and `tools/verify_keys.py` stay until the
Stage 9 cutover. `main` is frozen. Tag `pre-overhaul` is the rollback. An edit to a bullet goes into
both playbook homes until Stage 9, and a wrap does that by itself.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and `bun nv
selftest` runs every tools test. A scratch worktree needs `target/debug/nvs.exe` copied in and a
junction to `node_modules`. It has none of the git-ignored fixtures. Code copied into it is
uncommitted there, so a `git reset --hard` in the worktree silently puts the old tools back. Wrap
with `bun nv session --wrap`, not the pack's `python tools/session.py --wrap`: only the first also
writes `data/goals/tooling-overhaul.handoff.json`.

## Next group

**Stage 8: a goal switch copies nothing** — one file set: `tools/nv/lib/chain.ts`,
`tools/nv/cmd/loop.ts`, `tools/nv/cmd/session.ts`. loop-goal.md § *Stage 8* specifies it: "the live
goal is `.loop/state.sqlite`'s pointer, by slug", and Stage 9's `bun nv audit goals` wants no tracked
copy of the live goal.

- [ ] **The live goal is a pointer by slug.** `liveGoal` (`tools/nv/lib/chain.ts:69`) finds the goal
      whose H1 matches `docs/agent/loop-goal.md`. Read the slug from the driver's pointer instead, and
      keep the H1 match only as the fallback for a tree the driver has not run in. `nv loop`'s refusal
      at `tools/nv/cmd/loop.ts:146` names the copy.
- [ ] **The wrap reads one manifest.** `manifestCopies` (`tools/nv/cmd/session.ts:386`) gates the
      record and `docs/agent/loop-goal.toml`. Once nothing reads the copy, gate the record alone, and
      drop `GOAL_TOML` from `pruneManifests` (`tools/nv/cmd/playbook.ts:37`).
- [ ] **The pack is measured from `nv orient`.** `recordPack` (`tools/nv/cmd/session.ts:1512`)
      measures `python tools/orient.py`. Switch it to `bun tools/nv/main.ts orient` in the same
      commit that makes the driver pipe that pack. The two packs differ today by a few lines.

## Backlog

- `bun nv playbook` answers `--show` and `--retire` only. `--check`, `--match`, `--manifest` and
  `--dupes` are still `tools/playbook.py`'s (loop-goal.md § *Stage 8*).
- `nv goal context --add <path>` has no command yet (loop-goal.md § *Stage 8*).
