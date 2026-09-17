# Handoff

## State

**Goal `dossier` — queue the dossier — has just started; nothing of it has landed yet.** Goal `gap-zero`'s
whole acceptance list, which carries every goal before it, is this goal's floor. There is no design to settle: `rule:testing/four-proofs` decided the four proofs (the goal prose links
it; a handoff is copied to `docs/agent/` and its relative links would break),
`tools/dossier.py` derives the roster from `nvs meta --json`, and
`--emit-goals` puts the goals it writes onto the end of the chain the driver is walking. The last dry run
before this goal was reached said **1,069 features, four of them complete, 92 goals over 1,065 owed** —
read the numbers off your own run rather than trusting those.

## Next group

**The whole goal is one group** — one file set, `tools/dossier.py` and `docs/agent/goals/`. Stage 2 is
the emission and takes minutes; **stage 3 is the goal** — an optimization pass run at the one moment it
has leverage, with the 92 generated files in front of you and none of them walked yet. Do not stop after
stage 2 with headroom left.

- [ ] **`cargo build --release -p nvs-cli`**, then `python tools/dossier.py --emit-goals`. It prints what
      it wrote and the goal range it appended.
- [ ] **Read three or four of the generated `.toml`s** — one `Core` class, one `lang:` chapter, one
      `tools:` chapter. The three things to check are in `66-dossier.md`'s item list: the `[context]`
      manifest matches real modules, the `--group` argument spells the group the way `dossier.py` does,
      and the batch is a file set rather than an alphabetical run.
- [ ] **Any fix goes in `goal_toml()` / `goal_prose()` in `tools/dossier.py`**, then re-emit. A hand-edit
      to a generated file is lost at the next emission.
- [ ] **Two commits**: the generator fix, if there was one, and the generated tree.
- [ ] **Then stage 3**, which the goal prose owns in full. Four findings are already named there and
      measured — every goal's `[context] modules` resolving, the `--scaffold` question
      (answered: no, with the figure), what the growing floor actually costs, and the fan-out's width —
      plus whatever those four did not name. `python tools/dossier.py --check-goals` is the one
      mechanical gate; the rest of the stage lands in `.loop/optimization/report.md`, and its
      *Proposals* section is the valuable half.
- [ ] **The fan-out is already built and every emitted goal already drives it.** `dossier.py
      --partition --group G` writes the worker briefs, `--brief` prints one, `--findings` collates what
      they hit for one batch fix. Read `tools/dossier.py` § *Running one group's features at once*
      before stage 3 — the width is the one number in it that is still an estimate, and this session is
      where it stops being one.

## Backlog

- When this goal's checks go green the driver refreshes the chain and walks into the first emitted
  dossier goal — the one directly after this one — without a restart. `Chain.refresh()` in
  `tools/loop.py` is that half; if the console does not print `chain: docs/agent/goals/ is N goal(s)
  where it was M` after `GOAL REACHED`, that is the thing to look at, not the emitter.
- The proofs themselves start at the goal after this one and belong to no session of it. The one
  exception is
  stage 3: writing a single example and a single attack to feel the shape is a measurement, and it
  belongs in the report rather than in a commit.
- The emission is idempotent by slug and by claim: `--emit-goals --dry-run` says *nothing appended*
  while every owed feature is some generated goal's `--group` or `--only`, and that check rides in the
  floor of every generated goal after this one. It says *would append* — and the run halts on it — the
  day a feature lands that no goal on disk claims: a new member of a class split by `--only`, or a
  whole new group. The fix is one `python tools/dossier.py --emit-goals`; it appends one goal and the
  driver walks into it without a restart.
- A member whose implementing file moves owes its perf figure again. That shows first as its group's
  own floor check failing, and `--record-perf --group G` is the fix; until it lands, the dry-run check
  names the same feature.
