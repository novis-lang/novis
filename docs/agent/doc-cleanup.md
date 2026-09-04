# Documentation cleanup

How to run a cleanup pass over `docs/`. Revised 2026-08-27, after the pass that read all 102 ADRs
end to end; the 2026-08-23 and 2026-08-21 instruction sets are superseded, and most of what they asked
a reader to check by hand is now `python tools/adr.py`.

**Read this before starting, then start with the tool.** The 2026-08-27 pass spent its first third
answering questions the tool now answers in one call — which is exactly why the tool exists, and why
this file is shorter than the one it replaces.

## When a pass is due

**Not on a size signal — there is no size signal**, by design
([doc-style.md](doc-style.md) § *Length targets*). Doc length never triggers a pass, and nothing in
CI, `brief.py` or `orient.py` measures a doc against a number.

Run one when the **user** fires it, or when `python tools/adr.py` reports findings that accumulated
between passes. The tool is the standing signal; this file is the method for what it cannot decide.

## Start here, always

```
python tools/adr.py            # the audit: metadata, structure, links, § refs, amend symmetry,
                               #   index coverage, index freshness, changelog residue, stale counters
python tools/adr.py --stats    # size and section shape per ADR; the rationale column ranks the trim
python tools/adr.py --orphans  # what nothing links to, what is missing from an index, what is most cited
python tools/adr.py --graph NNNN   # one ADR's amend/cite graph, in both directions
python tools/check-links.py    # every link in every markdown file, case included
```

`adr.py` finding nothing does **not** mean the set is clean — it means eight mechanical failure classes
are absent. Duplication, contradiction and bloat are judgment, and the rest of this file is about those.

## The three constraints that decide what a pass may do

Every one of these was learned by nearly violating it. They are not preferences.

**1. An ADR number is a public identifier — merging is almost always wrong.** The ADRs are cited about
three thousand times from `crates/`, plus `tests/`, `docs/spec/`, `docs/plan/` and
`docs/agent/loop-goal.toml`. `grep -rhoE '\b0[01][0-9]{2}\b' crates/ | sort | uniq -c | sort -rn`
prints the weight: 0007 alone is cited 400+ times. A merge retires a number and invalidates every one
of those references, to save a few hundred lines that **trimming saves anyway**. The 2026-08-27 pass
evaluated five merge candidates that read as obviously mergeable — 0029+0030, and the four
rejected-PHP-spelling ADRs 0034/0045/0049/0050 — and rejected all five on this test alone. Fold an ADR
away only when it has **zero surviving unique content**, which 0032 had and nothing else has since.

**2. A section number is a public identifier too — never renumber one.** `0007 § 3`, `0066 § 3a` and
`0009 § 1` are cited from doc comments in `crates/`, from `docs/spec/01-core-library.md`, and from the
`[context]` manifest in `docs/agent/loop-goal.toml` that the running loop reads. Deleting `### 3.` and
promoting `### 4.` silently repoints every one of them. Insert as `§ 3a`; delete a section's *content*
and keep its number carrying the current rule. `adr.py`'s **section refs** check catches the citations
that are already wrong, not the ones a renumber would create.

**3. The loop may be running while you work.** `tools/session.py --wrap` stages a slice's own files and
lets the last commit sweep whatever is left, so an uncommitted docs change can be swept into a loop
commit. Confine a pass to `docs/` and `tools/`, check `git log --oneline -3` before and after, and
commit promptly rather than leaving a large working tree open across loop iterations.

## What a pass removes, in priority order

**1. Changelog prose in a body.** This is the highest-value removal and `--residue` finds most of it.
An ADR body states the current rule and nothing else; git holds the history. The shapes it takes:

- a tombstone section — `### 3. Withdrawn — traits do not exist`, whose whole content was an account
  of what the section used to say. Keep the number, replace the content with the current rule.
- a parenthetical narrating an edit — *"(This body previously said …, which read as narrowing-only)"*.
- a section restating another ADR's table plus the edit made to it.
- a `Consequences` bullet describing a change made to a *different* ADR in the same commit.
- the `Amended by: 0011, 0064 — each fold is applied below; this body states the current rule.`
  boilerplate, which was in 30 files and which README.md already states once for the whole set.

**2. A count kept in more than one file.** Always wrong somewhere. The 2026-08-27 pass found two: the
PHP-divergence count, which stopped at "the twelfth" while sixteen more ADRs added rows, and the
compiler-recognized-attribute count, which 0102 § 9 had already found wrong in seven places. The fix
is a register with one home — [divergences.md](../adr/divergences.md) is now that home for the first —
and prose that says "joins the closed list" without a number. `--only "stale counters"` looks for the
spellings; a new kind of count needs a new pattern in `COUNTERS`.

**3. A derivable index maintained by hand.** README's index table's *Decision* cell was a second copy
of each ADR's own title, and had drifted: 26 of 102 cells past 200 bytes, one at 836. It is generated
now (`python tools/adr.py --index`) and checked. Look for the same shape elsewhere before writing a
new list by hand: if a fact can be derived from the files, derive it.

**4. Rationale bulk — the largest remaining target, and still untouched.** `Context`,
`Investigation`, `Options considered`, `Alternatives rejected` and `Revisiting` are **24% of the ADR
corpus, 477 KB across 1.95 MB**, at full prose length. `--stats`' rationale column ranks them; as of
2026-08-27 the top of that list is 0100, 0043, 0047, 0033, 0079, 0040, 0048, 0101, 0042 and 0102, each
carrying 7–11 KB.

The trim depth, when a pass takes this on:

- Keep the metadata block's *fields* and cut its prose — an `Amends:` is one clause per target.
- Change `## Decision` and `## Consequences` only to make them state the current rule.
- Compress `Context` / `Investigation` / `Alternatives rejected` into short bullet lists: one bullet
  per alternative, name plus a one-phrase reason. Target 5–15 lines per section rather than 40–100.
- Delete extended comparison tables, prior-art surveys ("how Go/Rust/Java handle this") and
  multi-paragraph reasoning chains down to a one-line callout, or cut them entirely.
- Where a choice diverges from PHP, state that it was decided and move on. The reasoning belongs in
  one place, and [divergences.md](../adr/divergences.md) is where the *fact* is indexed.
- If a section has nothing left worth a bullet, delete the heading too — but see constraint 2 if it
  is numbered.

**Do it whole or not at all.** A half-trimmed corpus is less consistent than an untrimmed one, and
consistency is most of what a reader is buying. This is a pass of its own, sized against `--stats`.

## What a pass never does

- **Never answers an open question.** A fork waiting on a measurement waits for whoever takes the
  measurement. What a pass *may* do is narrow a status field to what is actually open.
- **Never touches `docs/agent/loop-goal.md`, `loop-goal.toml`, `handoff.md`, or the plan's status
  block.** Those are the running loop's state.
- **Never renumbers an ADR or a section.** Constraints 1 and 2.
- **Never trims the routing table or `ground-rules.md` for size.** Both grow a row per decision and
  are supposed to; they are the only way anything is found.
- **Never changes what a `## Decision` decides** — only what it says the current rule is.

## The judgment half: what the tool cannot see

Read for these directly; there is no check for any of them.

- **Conceptual duplication.** Textual duplication is essentially absent in this corpus — a
  normalized-sentence sweep across all 102 ADRs found five duplicate sentences, all of them the
  boilerplate above. What recurs instead is one *rule* explained in two places, usually because one
  ADR restates a neighbour's table "because this is the boundary a reader lands on first". Decide
  which is the home, and make the other a link.
- **A body that disagrees with a cross-link.** `--only "amend symmetry"` finds the missing link, not
  the stale prose behind it. 68 of the 102 ADRs had an asymmetric amend record before the 2026-08-27
  pass; every one of those was a place where a fold might have been recorded on one side only.
- **A doc-vs-code disagreement.** If the code shipped something an ADR still calls Proposed, or carries
  a type the ADR describes and the compiler never had, say so and fix it. Flag the call to the user
  rather than making it silently.
- **A claim with no home.** `divergences.md` exists because "the tenth deliberate divergence" was a
  fact nobody owned. When a pass finds prose counting or summarising across ADRs, that is the signal
  for a register, not for better prose.

## Finishing

1. `python tools/adr.py` and `python tools/check-links.py` both clean. There is no build to break —
   this is a docs pass, and a broken or mis-cased cross-link is the only mechanical risk.
2. If the pass changed the ADR set's *shape*, update this file and `tools/adr.py`'s draft in the same
   commit — the draft is the skeleton's only copy, and [conventions.md](conventions.md) § *An ADR*
   points at it rather than restating it. A rule the tool enforces is documented once, in the
   skeleton; a rule it cannot enforce is documented here.
3. Commit with line counts before and after in the message.
4. Follow [AGENTS.md](../../AGENTS.md)'s session workflow: overwrite [handoff.md](handoff.md) — unless
   a loop is mid-goal, in which case say so in chat and leave that file to the loop.
