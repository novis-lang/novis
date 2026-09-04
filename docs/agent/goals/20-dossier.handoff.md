# Handoff

## State

**Goal 20 — queue the dossier — has just started; nothing of it has landed yet.** Goal 19's whole list is
this goal's floor. There is no design to settle: [ADR 0134](../adr/0134-every-shipped-feature-owes-four-proofs.md)
decided the four proofs, `tools/dossier.py` derives the roster from `nvs meta --json`, and
`--append-chain` puts the goals it writes onto the end of the chain the driver is walking. The last sweep
before this goal was written said **795 features, one of them complete, 93 goals over 794 owed** — read
the numbers off your own run rather than trusting those.

## Next group

**The whole goal is one group** — one file set, `tools/dossier.py` and `docs/agent/goals/`.

- [ ] **`cargo build --release -p nvs-cli`**, then
      `python tools/dossier.py --emit-goals --append-chain docs/agent/goals/chain.toml`. It prints what
      it wrote and the goal range it appended.
- [ ] **Read three or four of the generated `.toml`s** — one `Core` class, one `lang:` chapter, one
      `tools:` chapter. The three things to check are in `20-dossier.md`'s item list: the `[context]`
      manifest matches real modules, the `--group` argument spells the group the way `dossier.py` does,
      and the batch is a file set rather than an alphabetical run.
- [ ] **Any fix goes in `goal_toml()` / `goal_prose()` in `tools/dossier.py`**, then re-emit. A hand-edit
      to a generated file is lost at the next emission.
- [ ] **Two commits**: the generator fix, if there was one, and the generated tree.

## Backlog

- When this goal's checks go green the driver refreshes the chain and walks into goal 21 — the first
  emitted dossier goal — without a restart. `Chain.refresh()` in `tools/loop.py` is that half; if the
  console does not print `chain: docs/agent/goals/chain.toml grew by N goal(s)` after `GOAL REACHED`,
  that is the thing to look at, not the emitter.
- The proofs themselves start at goal 21 and belong to no session of this one.
