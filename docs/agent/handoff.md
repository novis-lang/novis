# Handoff

## State

**Goal `gap-zero` — no gap is owed by anyone but a future milestone, and the index that held them is
gone — has just started; nothing of it has landed yet.** Goal `unowned-closures`'s whole list is this
goal's Stage 1 floor, and it holds every closure goal's checks.

Settled before the first session: **this goal builds nothing.** If `python tools/owners.py --check` or
`python tools/plan.py --past` names an open item, a closure goal left it behind — that is a `BLOCKED`
naming the item, never a new owner or a deferral invented to pass. The one other expected hold is CI:
if GitHub still refuses to start jobs, stage 4 is a `BLOCKED` naming the billing block.

## Next group

**Stage 2 and stage 3: the fatal gate, then the index deleted** — one file set: `tools/owners.py`,
`tools/playbook.py`, `tools/brief.py`, `crates/nvs-stdlib/tests/spec_registry_coverage.rs`.

- [ ] **Retire `unowned`** — `tools/owners.py:@classify`, `tools/owners.py:@tag_of`, and
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:433`'s `owner_problem`.
- [ ] **The full gate by default, run by `verify.py`** — `tools/owners.py:@run_check`, `tools/verify.py`.
- [ ] **Delete `docs/agent/carried-gaps.md` and re-point every reader in the same slice** — the list is
      in the goal prose; `python tools/check-links.py` is the proof.

## Backlog

- Stage 4, CI green on `main` — `gh run list --branch main --workflow ci.yml`.
- Stage 5, `python tools/plan.py --past` and `--sync` writing `done`; the status block's *Done* field.
- When this goal's last check goes green the driver takes goal `dossier`.
