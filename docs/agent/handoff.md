# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, Stage 4's records are valid, and Stage 8's
checks are green. Stage 8's prose still owes the handoff record from `nv session --wrap`, the
playbook retirement in TypeScript, and the copy-free goal switch.** `bun nv check` prints
`nv check: 0 findings`, and `bun nv render --check` is current. The driver's red check is Stage 9's
`bun nv audit goals`, which is the cutover and is not started.

`bun nv session --wrap` now validates and applies a wrap by itself: `applyWrap` in
`tools/nv/cmd/session.ts` is `tools/session.py`'s `wrap`, ported with every `apply_*`,
`playbook_targets` and `playbook_collisions`. A plan field is written to the plan's Markdown and to
`data/plan/status.json`. A playbook bullet goes to its fragment file and to its record under
`data/playbook/`. Applied for real in a scratch worktree, every byte under `docs/` matched what
`tools/session.py` writes on the same base, and both records matched what `nv import` builds. Two
parts still call Python: `retireExpired` runs `tools/playbook.py`'s `expiry_report` and `retire`,
and `recordPack` measures `python tools/orient.py`, which is the pack the driver pipes in.
`bun nv parity writers` is 8 of 8, including a valid wrap under `--dry-run`.

The collision check names a lead-in by its whole bold text, as `nv orient` reads it through the
importer. `tools/session.py` cuts a lead-in that wraps across lines at 78 characters, so the two
tools name a different selector in the same refusal. No parity case covers a collision, because a
fixture that collides with a live bullet breaks when that bullet retires.

Seven bullet records under `data/playbook/` disagree with their fragment files (the `tools/verify.py`
and `tools/splice.py` bullets among them). This was already the case before this session.

`tools/orient.py`, `chain.py`, `session.py` and `tools/verify_keys.py` stay until the Stage 9
cutover. `main` is frozen. Tag `pre-overhaul` is the rollback. An edit to a bullet goes into both
playbook homes until Stage 9, and a wrap now does that by itself.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and
`bun nv selftest` runs every tools test.

## Next group

**Stage 8: the writers and the driver** — one file set: `tools/nv/cmd/session.ts`, `tools/nv/import/goals.ts`, `tools/nv/import/playbook.ts`. loop-goal.md § *Stage 8* specifies it: "The handoff section writes the handoff record", and the playbook section "writes bullet records with `files` and `until`".

- [ ] **The handoff section writes the handoff record.** `applyHandoff`
      (`tools/nv/cmd/session.ts:1430`) writes only the Markdown. Build the record with `handoffValue`
      (`tools/nv/import/goals.ts:241`) after the Markdown is written, and add its path to
      `writtenPaths`. Prove it as this session did: a real apply in a detached worktree under
      `.agent-tmp/worktrees/`, then `nv import`'s records compared with `data/`.
- [ ] **Resync the seven stale bullet records.** Compare each record under `data/playbook/` with
      what `bulletValue` (`tools/nv/import/playbook.ts:51`) builds from its fragment file, and
      rewrite the ones that differ, with a script under `.agent-tmp/`.
- [ ] **Retirement in TypeScript.** Port `expiry_report` and `retire` (`tools/playbook.py:547`) and
      replace the Python bridge in `retireExpired` (`tools/nv/cmd/session.ts:1507`).

## Backlog

- The copy-free goal switch: Stage 8's prose in `docs/agent/loop-goal.md`.
- `recordPack` (`tools/nv/cmd/session.ts:1553`) measures `python tools/orient.py`; it moves to `nv orient` when the driver does.
- Stage 9, the cutover: `bun nv audit goals`, `audit checks`, `audit eol` (loop-goal.toml Stage 9).
