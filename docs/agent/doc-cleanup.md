# Documentation cleanup

How to run a cleanup pass over `docs/`. It was rewritten after the pass that read every record end to
end, and again when the docs migration froze the records and moved the rules into the rulebook; the
instruction sets it replaces are superseded, and most of what they asked a reader to check by hand is
now `bun nv records` and `bun nv rules --check`.

**Read this before starting, then start with the tools.** An earlier pass spent its first third
answering questions the tools now answer in one call — which is exactly why they exist, and why
this file is shorter than the one it replaces.

## When a pass is due

**Not on a size signal — there is no size signal**, by design
([doc-style.md](doc-style.md) § *Length targets*). Doc length never triggers a pass, and nothing in
CI, `bun nv brief` or `bun nv orient` measures a doc against a number.

Run one when the **user** fires it, or when `bun nv records` or `bun nv rules --check`
reports findings that accumulated between passes. The tools are the standing signal; this file is the
method for what they cannot decide.

## Start here, always

```
bun nv records              # the record audit: metadata, changes, structure, links, § refs, residue, stale counters
bun nv records --stats      # size and section shape per record
bun nv records --orphans    # what nothing links to, and what is most cited
bun nv records --graph NNNN # what one record created and modified, and who else touched those rules
bun nv rules --check        # the rulebook: every id, fragment, `because`, `seeAlso`, guard and citation
bun nv links                # every link in every tracked file, case included
```

The tools finding nothing does **not** mean the set is clean — it means the mechanical failure classes
are absent. Duplication, contradiction and bloat are judgment, and the rest of this file is about those.

## The three constraints that decide what a pass may do

Every one of these was learned by nearly violating it. They are not preferences.

**1. A record number is a public identifier, and a record is frozen — merging is never right.** The
records are cited about three thousand times from `crates/`, plus `tests/`, `docs/spec/`, `docs/plan/`
and `docs/agent/loop-goal.toml`, and every rule's `because` names them. `grep -rhoE '\b0[01][0-9]{2}\b'
crates/ | sort | uniq -c | sort -rn` prints the weight: 0007 alone is cited 400+ times. A merge retires
a number and invalidates every one of those references. An earlier pass evaluated five merge
candidates that read as obviously mergeable — 0029+0030, and the four rejected-PHP-spelling records
0034/0045/0049/0050 — and rejected all five on this test alone; 0032 was folded away before the freeze,
and nothing has been since. A record's body is rationale as of its date and is not edited to say
something new — what is currently true is the rule's fragment under `docs/rules/`.

**2. A section number is a public identifier too — never renumber one.** `0007 § 3`, `0066 § 3a` and
`0009 § 1` are cited from doc comments in `crates/`, from `docs/spec/01-core-library.md` — which is
live, read at test time by `crates/nvs-stdlib/tests/spec_registry_coverage.rs` and by
`bun nv migration` — and from the `[context]` manifest in `docs/agent/loop-goal.toml` that the
running loop reads. Deleting `### 3.` and promoting `### 4.` silently repoints every one of them.
Insert as `§ 3a`; delete a section's *content* and keep its number. `bun nv records`' **section refs** check
catches the citations that are already wrong, not the ones a renumber would create.

**3. The loop may be running while you work.** `bun nv session --wrap` stages a slice's own files and
lets the last commit sweep whatever is left, so an uncommitted docs change can be swept into a loop
commit. Confine a pass to `docs/` and `tools/`, check `git log --oneline -3` before and after, and
commit promptly rather than leaving a large working tree open across loop iterations.

## What a pass removes, in priority order

**1. Changelog prose in a rule's body.** This is the highest-value removal. A rule's fragment under
`docs/rules/` states the current rule and nothing else; git holds the history, and the records its
`because` names hold the reasoning. The shapes it takes:

- a tombstone paragraph whose whole content is an account of what the rule used to say. Replace it with
  the current rule.
- a parenthetical narrating an edit — *"(This previously said …, which read as narrowing-only)"*.
- a fragment restating another rule's table plus the edit made to it.
- a sentence describing a change made to a *different* rule in the same commit.

A frozen record is exempt: its body was true on its date, and a pass leaves it alone.

**2. A count kept in more than one file.** Always wrong somewhere. An earlier pass found two: the
PHP-divergence count, which stopped at "the twelfth" while sixteen more records added rows, and the
compiler-recognized-attribute count, which 0102 § 9 had already found wrong in seven places. The fix
is a register with one home — [divergences.md](../divergences.md), generated from every rule carrying
`divergesFromPhp`, is now that home for the first — and prose that says "joins the closed list" without
a number. `--only "stale counters"` looks for the spellings; a new kind of count needs a new pattern in
`COUNTERS`.

**3. A derivable index maintained by hand.** README's index table's *Decision* cell was a second copy
of each record's own title, and had drifted: 26 of 102 cells past 200 bytes, one at 836. That table is
gone, and so are the authored `ground-rules.md` and `divergences.md`: `bun nv rules --render`
writes both and every `docs/rules/<topic>.md` chapter from the topic JSON and the fragments, and
`--check` reports a copy that has drifted. Look for the same shape elsewhere before writing a new list
by hand: if a fact can be derived from the files, derive it.

## What a pass never does

- **Never answers an open question.** A fork waiting on a measurement waits for whoever takes the
  measurement. What a pass *may* do is narrow a status field to what is actually open.
- **Never touches `docs/agent/loop-goal.md`, `loop-goal.toml`, `handoff.md`, or the plan's status
  block.** Those are the running loop's state.
- **Never renumbers a record or a section.** Constraints 1 and 2.
- **Never edits a generated file.** `docs/ground-rules.md`, `docs/divergences.md` and
  `docs/rules/<topic>.md` are written by `bun nv rules --render`; an edit there is lost on the
  next render, and the file that lost it does not say so. Edit the fragment or the topic JSON.
- **Never changes what a rule decides** — only what its fragment says the current rule is. A change
  of substance is a new decision record.

## The judgment half: what the tool cannot see

Read for these directly; there is no check for any of them.

- **Conceptual duplication.** Textual duplication is essentially absent in this corpus — a
  normalized-sentence sweep across all 102 records found five duplicate sentences, all of them
  boilerplate. What recurs instead is one *rule* explained in two places, usually because one
  fragment restates a neighbour's table "because this is the boundary a reader lands on first". Decide
  which is the home, and make the other a `rule:` citation.
- **A fragment that disagrees with the record behind it.** `bun nv rules --check` finds a `because` that
  names nothing, not a fragment whose text no longer says what its latest record decided. Read the
  last record in the rule's `because` against the fragment when either looks suspect.
- **A doc-vs-code disagreement.** If the code shipped something a rule still calls designed, or carries
  a type a rule describes and the compiler never had, say so and fix it. Flag the call to the user
  rather than making it silently.
- **A claim with no home.** `divergences.md` exists because "the tenth deliberate divergence" was a
  fact nobody owned. When a pass finds prose counting or summarising across rules, that is the signal
  for a register, not for better prose.

## Finishing

1. `bun nv records --check`, `bun nv rules --check` and `bun nv links` all clean.
   There is no build to break — this is a docs pass, and a broken or mis-cased cross-link is the only
   mechanical risk.
2. If the pass changed the record set's *shape*, update this file and [conventions.md](conventions.md)
   § *A decision record* in the same commit — that section is the shape's only copy, and
   `tools/nv/cmd/records.ts`'s `CANONICAL` list is the heading set it names. A rule the tool enforces is documented
   once, there; a rule it cannot enforce is documented here.
3. Commit with line counts before and after in the message.
4. Follow [AGENTS.md](../../AGENTS.md)'s session workflow: overwrite `handoff.md` — unless
   a loop is mid-goal, in which case say so in chat and leave that file to the loop.
