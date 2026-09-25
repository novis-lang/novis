---
milestone: dossier
---
# Loop goal 172 — the description is owed

The checks are the goal's record, `data/goals/the-description-is-owed.json`; this half is the target and the
standing decisions.

## The target

Every goal before this one wrote each feature's website description -- `about.md` in its
example directory -- without any check counting it. **This goal makes it owed**, for every
kind of feature, and closes whatever the goals before it left. When it is reached, a
feature without its description is a red check for every goal that follows, the same as
a feature without its test, and `rule:testing/feature-proofs`'s proofs are what
finished means in this repository.

## The item list

One file set -- the policy file and the example tree -- so this is one group.

- [ ] **Switch it on.** Add `"about": true` under `owes.all` in `data/proofs/policy.json`,
      creating `owes` if the file has none. Nothing in `tools/nv/proofs/collect.ts`
      changes: `POLICY` stays the default and the file is the repository's durable
      answer, exactly as it is for `perf`.
- [ ] **Read what is left.** `python tools/dossier.py --owed` now lists every feature
      with no description, and every description outside 40 to 200 words, opening
      with a heading, or carrying a code block.
- [ ] **Close it, one feature at a time.** Read the feature's examples first -- they are
      already on disk here, so the description is written to fit them, and where it
      closes with `**The examples below**` it names what they really show, in their
      order. `python tools/dossier.py --partition --group <G>` fans a large group out.

## Stage 2 — the description is owed

**Does:** Makes a feature's `about.md` description owed, and writes every one still missing.

## Standing decisions

- **`docs/examples/README.md` § *The description* is the standard**, and it is not
  reopened here: plain prose a beginner and an expert read the same way, no code, an
  `**In plain words:**` picture only where the explanation is technical.
- **A description that fails the shape check is rewritten, never padded or trimmed to
  fit.** Too long means it is explaining edge cases the tests own; too short means the
  lead sentence is carrying the whole page.
- **A feature that cannot carry a description does not exist.** Every feature has a page
  on the website, so there is no `[skip]` entry for `about`.
- **No numbered ADR is opened by this goal.** The decision is `rule:testing/feature-proofs`.

