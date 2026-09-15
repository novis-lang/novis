# Handoff

## State

**Goal `m8-db-queue` is met.** `python tools/loop.py --goal-only` ends `GOAL REACHED: every acceptance
check passes` (782 checks remembered, 7 re-run), and `python tools/verify.py --doc` resolves every link —
the one gate a goal meets only at its end.

The check this session was handed, `no module-doc gap names a goal that walked without closing it`, was
**already green on arrival**: `927195406` re-owned goal `m7-server-surface`'s three open gaps by hand
while the run was held, and `cab85530d` is the `tools/loop.py` change that grants a retry when a DONE
falls to a *different* check. The playbook already carries that trap — *the pack's "THE DRIVER'S LAST
ACCEPTANCE CHECK FAILED" line survives a hold* — and this session did exactly what it says: ran the
named `argv` once, found exit 0, collected the acceptance and re-claimed. No new bullet.

No code changed this session. Nothing is blocked.

## Next group

**Goal `unowned-closures` stage 2: the lowering and the runtime, security first** — one file set:
`crates/nvs-ir/src/lower/` and `crates/nvs-runtime/src/`. A goal switch reseeds the handoff from
`docs/agent/goals/60-unowned-closures.handoff.md`, so this trio is repeated here only so that a refused
DONE does not lose it. Each item's answer is the `Decided:` sentence already on disk under its gap;
build to it and never re-open it.

- [ ] **A `secret` compared against a `mixed` is constant-time** —
      `crates/nvs-ir/src/lower/operator.rs:896`, with the type side at `crates/nvs-ir/src/lib.rs:405`.
- [ ] **One allocation past the budget, to its decision** — `crates/nvs-runtime/src/budget.rs:89`.
- [ ] **A hooked property is reached through an erased key** — `crates/nvs-runtime/src/object.rs:3403`.

## Backlog

- The db half session 0004 scoped, now stage 4 of `unowned-closures`: `crates/nvs-stdlib/src/db/mod.rs:214`
  gap 1 wants the first-use notice or the record that there is none, and its neighbour gap 2 the `DbError`
  split — both `Decided:` on disk.
- `crates/nvs-types/src/derive.rs:44` gap 1, the third of that group — `rule:core-classes/derive-attribute`.
- Stages 3, 5 and 6 of the next goal, listed in its own seeded handoff — `docs/agent/goals/60-unowned-closures.handoff.md`.
- What must survive the goal switch is `docs/agent/carried-gaps.md`, not this file.
