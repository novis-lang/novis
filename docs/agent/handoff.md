# Handoff

## State

**Goal `class-scoped-types` is complete, and the hold on the run is lifted.** All five stages are
landed and green; what held the run was the floor's `no module-doc gap names a goal that walked
without closing it`, red on forty items goal `unowned-closures` had tagged to itself and never built.

**The hold was a driver bug, and it is fixed.** That floor check reads a goal as retired off its
`.toml` being gone, which `chain.py --retire` does *after* the goal is reached, so an item tagged to
the reaching goal is goal-owned on the sweep that reaches it and retired-owner one goal later. The
goal's own gate was `owners.py`'s `unowned: 0`, which tagging alone satisfies. Now `tools/loop.py`'s
`owner_gate` runs `python tools/owners.py --closes <slug>` and `python tools/playbook.py --closes
<slug>` on the sweep that would reach a goal, beside the rustdoc gate, and holds the goal open while
either names a gap; `orient.py` prints the finding under *THE OWNER GATE IS RED*.

**The forty-eight items have an owner again**: goal `decided-closures`, inserted at 66 of 68 in front
of `gap-zero`, takes the forty with their `Decided:` sentences and the eight owed to M1, M6, M7 and
M8 that no live goal held. Its gate is `--closes`, which a tag cannot meet. `python tools/owners.py`
reads `retired-owner: 0` and `past-milestone: 0`, `plan.py --sync` marked M1, M6 and M8 `done`, and
`chain.py --check` and `plan.py --check` are clean.

## Next group

**Claim the goal** — one file set: `docs/agent/goals/61-class-scoped-types.toml`.

- [ ] **Run the two goal-end gates and write `DONE`** — `docs/agent/goals/61-class-scoped-types.toml:9483`
      is the floor check that held the run, green now; `python tools/verify.py --doc`, then `python
      tools/owners.py --closes class-scoped-types` (owns none) and `python tools/playbook.py --closes
      class-scoped-types` (owns none) are what the driver asks before it switches to goal
      `worker-placement`.

## Backlog

- `docs/rules/types/type-alias.md` still describes only the file-scope form's `use`/FQN resolution
  in its third paragraph; the class-scoped site's resolution is in `rule:types/class-scoped-alias`.
- Goal `decided-closures`'s `[context] modules` names 33 files, one per file holding an item it owns;
  the driver will say it would pass at 18. Narrowing it is a person's call once the first sessions
  show which stage's files are opened together.
- `crates/nvs-host/src/net.rs:1792` is a race, not a regression: the test binds port 0, drops the
  listener and connects, and a sibling test binary can take that port in between. It failed once
  beside the other binaries this session and passed alone; a retry on a stolen port would close it.
