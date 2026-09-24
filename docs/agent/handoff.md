# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, Stage 4's records are valid, and Stage 8's
checks are green. Stage 8's prose still owes the playbook retirement in TypeScript and the copy-free
goal switch.** `bun nv check` prints `nv check: 0 findings`, and `bun nv render --check` is current.
The driver's red check is Stage 9's `bun nv audit goals`, which is the cutover and is not started.

`bun nv session --wrap` validates and applies a wrap by itself. `applyWrap` in
`tools/nv/cmd/session.ts` is `tools/session.py`'s `wrap`, ported whole. A plan field is written to
the plan's Markdown and to `data/plan/status.json`. A playbook bullet is written to its fragment file
and to its record. The handoff is written to its Markdown and to `data/goals/<slug>.handoff.json`
(`data/goals/side/` for a side goal), and nothing is recorded while no goal is live. Applied for real
in a scratch worktree, the handoff record matched what `nv import` builds from the same Markdown.
`bun nv parity writers` is 8 of 8, and it declares that record as nv's one extra path. Two parts
still call Python: `retireExpired` runs `tools/playbook.py`'s `expiry_report` and `retire`, and
`recordPack` measures `python tools/orient.py`.

Every playbook record now has the same `lead`, `body` and `until` as its fragment file. Eight differ
in `files` alone, and there the record is the right side: it also names the TypeScript tool that
replaced the Python one. The importer reads `files` only from paths the prose cites, so
`nv import --write` still rewrites those eight. The bullet
`docs/agent/playbook/tooling/bun-nv-import-write-rewrites-a-data-playbook-bullet-back-to.md` says what
to do.

`tools/orient.py`, `chain.py`, `session.py` and `tools/verify_keys.py` stay until the Stage 9
cutover. `main` is frozen. Tag `pre-overhaul` is the rollback. An edit to a bullet goes into both
playbook homes until Stage 9, and a wrap does that by itself.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and
`bun nv selftest` runs every tools test. A scratch worktree needs `target/debug/nvs.exe` copied in,
or the wrap's `nv reference --check` refuses, and it has none of the git-ignored fixtures, so the
`files` of a record that cites one differ there and nowhere else.

## Next group

**Stage 8: the writers and the driver** — one file set: `tools/nv/cmd/session.ts`, `tools/playbook.py`, `tools/nv/cmd/orient.ts`. loop-goal.md § *Stage 8* specifies it: the playbook section "writes bullet records with `files` and `until`", and the goal switch copies no file.

- [ ] **Retirement in TypeScript.** Port `expiry_report` and `retire` (`tools/playbook.py:547`) and
      replace the Python bridge in `retireExpired` (`tools/nv/cmd/session.ts:1528`). A retired bullet
      deletes its fragment file and its record. Prove it with a bullet whose trailer has already
      expired, applied for real in a scratch worktree.
- [ ] **The pack is measured from `nv orient`.** `recordPack` (`tools/nv/cmd/session.ts:1574`)
      measures `python tools/orient.py`. Switch it once the driver pipes `nv orient`, and not before,
      because it measures the pack the driver sends.

## Backlog

- The copy-free goal switch: Stage 8's prose in `docs/agent/loop-goal.md`.
- Stage 9, the cutover: `bun nv audit goals`, `audit checks` and `audit eol` in `docs/agent/loop-goal.toml`.
