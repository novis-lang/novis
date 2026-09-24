## plan-edit: Open now
--- old
the corpus and bench figures are `python tools/disk.py`'s and the perf notes'.
--- new
the corpus and bench figures are the disk report's and the perf notes'.

## milestone: M8
A milestone body this fixture never writes: it is applied only under `--dry-run`.

**Verify:** the parity case prints the same line on both sides.

## playbook: Tooling
- **A parity fixture wrap is applied only under `--dry-run`.** `tools/nv/parity/fixtures/session-valid-wrap.md` names real files, so a run without the flag would commit them. [until: gone tools/nv/parity/fixtures/session-valid-wrap.md:dry-run]

## handoff
## State

A handoff this fixture never writes.

## Next group

**Stage 8: the writers** — one file set: `tools/nv/cmd/session.ts`.

- [ ] **An item.** Anchored at `tools/nv/cmd/session.ts:1`.

## Backlog

- Nothing.

## commit: docs/implementation-plan.md data/plan docs/plan docs/agent/playbook data/playbook docs/agent/handoff.md
docs(agent): a wrap that validates and is only ever applied under a dry run

## status
CONTINUE a parity fixture that is never applied
