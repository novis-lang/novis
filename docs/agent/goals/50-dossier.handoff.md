# Handoff

## State

**Goal 50 — queue the dossier — has just started; nothing of it has landed yet.** Goal 20's whole list is
this goal's floor. There is no design to settle: ADR 0134 decided the four proofs (the goal prose links
it; a handoff is copied to `docs/agent/` and its relative links would break),
`tools/dossier.py` derives the roster from `nvs meta --json`, and
`--append-chain` puts the goals it writes onto the end of the chain the driver is walking. The last sweep
before this goal was written said **795 features, one of them complete, 93 goals over 794 owed** — read
the numbers off your own run rather than trusting those.

## Next group

**The whole goal is one group** — one file set, `tools/dossier.py` and `docs/agent/goals/`. Stage 2 is
the emission and takes minutes; **stage 3 is the goal** — an optimization pass run at the one moment it
has leverage, with the 93 generated files in front of you and none of them walked yet. Do not stop after
stage 2 with headroom left.

- [ ] **`cargo build --release -p nvs-cli`**, then
      `python tools/dossier.py --emit-goals --append-chain docs/agent/goals/chain.toml`. It prints what
      it wrote and the goal range it appended.
- [ ] **Read three or four of the generated `.toml`s** — one `Core` class, one `lang:` chapter, one
      `tools:` chapter. The three things to check are in `50-dossier.md`'s item list: the `[context]`
      manifest matches real modules, the `--group` argument spells the group the way `dossier.py` does,
      and the batch is a file set rather than an alphabetical run.
- [ ] **Any fix goes in `goal_toml()` / `goal_prose()` in `tools/dossier.py`**, then re-emit. A hand-edit
      to a generated file is lost at the next emission.
- [ ] **Two commits**: the generator fix, if there was one, and the generated tree.
- [ ] **Then stage 3**, which the goal prose owns in full. Three findings are already named there and
      measured — 14 goals whose `[context] modules` `orient.py` cannot map, whether a `--scaffold` is
      worth building, and what the growing floor actually costs — plus whatever those three did not
      name. `python tools/dossier.py --check-goals` is the one mechanical gate; the rest of the stage
      lands in `.loop/optimization/report.md`, and its *Proposals* section is the valuable half.

## Backlog

- When this goal's checks go green the driver refreshes the chain and walks into goal 51 — the first
  emitted dossier goal — without a restart. `Chain.refresh()` in `tools/loop.py` is that half; if the
  console does not print `chain: docs/agent/goals/chain.toml is N goal(s) where it was M` after
  `GOAL REACHED`,
  that is the thing to look at, not the emitter.
- The proofs themselves start at goal 51 and belong to no session of this one. The one exception is
  stage 3: writing a single example and a single attack to feel the shape is a measurement, and it
  belongs in the report rather than in a commit.
