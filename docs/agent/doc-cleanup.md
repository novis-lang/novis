# Documentation cleanup

How to run a cleanup pass over `docs/`. Most of what a pass would check by hand is `bun nv rules
--check` and `bun nv links`; this file is the method for what they cannot decide.

**Read this before starting, then start with the tools.** They answer the mechanical questions in one
call, and a pass that answers them by hand spends its first third doing so.

## When a pass is due

**Not on a size signal — there is no size signal**, by design
([doc-style.md](doc-style.md) § *Length targets*). Doc length never triggers a pass, and nothing in
CI, `bun nv brief` or `looper orient` measures a doc against a number.

Run one when the **user** fires it, or when `bun nv rules --check` reports findings that accumulated
between passes. The tools are the standing signal.

## Start here, always

```
bun nv rules --check        # the rulebook: every id, fragment, `seeAlso`, guard and citation, and no decision record
bun nv rules --stats        # one line per topic: rules, bytes, status split
bun nv links                # every link in every tracked file, case included
```

The tools finding nothing does **not** mean the set is clean — it means the mechanical failure classes
are absent. Duplication, contradiction and bloat are judgment, and the rest of this file is about those.

## The two constraints that decide what a pass may do

**1. A rule id is a public identifier — never rename one in a cleanup pass.** `rule:<topic>/<slug>` is
cited from `crates/`, `tests/`, `docs/spec/` and the goal files under `docs/goals/`.
`bun nv rules --citations` prints every one. A rename repoints all of them, and `bun nv rules --check`
catches only the ones left dangling. A rule is edited so its body is true now; it is not merged into a
neighbour to save a file.

**2. The loop may be running while you work.** `looper wrap` stages a slice's own files and
lets the last commit sweep whatever is left, so an uncommitted docs change can be swept into a loop
commit. Confine a pass to `docs/` and `tools/`, check `git log --oneline -3` before and after, and
commit promptly rather than leaving a large working tree open across loop iterations.

## What a pass removes, in priority order

**1. Changelog prose in a rule's body.** This is the highest-value removal. A rule's fragment under
`docs/rules/` states the current rule, its reasons and its Declined list, and nothing else; git holds
the history. The shapes changelog prose takes:

- a tombstone paragraph whose whole content is an account of what the rule used to say. Replace it with
  the current rule.
- a parenthetical narrating an edit — *"(This previously said …, which read as narrowing-only)"*.
- a fragment restating another rule's table plus the edit made to it.
- a sentence describing a change made to a *different* rule in the same commit.

**2. A count kept in more than one file.** Always wrong somewhere: a count of deliberate PHP
divergences or of compiler-recognized attributes goes stale the moment a rule adds one and another file
keeps the old figure. The fix is prose that says "joins the closed list" without a number, and where a
count is really needed, a register generated from the files that hold the items.

**3. A derivable index maintained by hand.** `ground-rules.md` and every `docs/rules/<topic>.md`
chapter are written by `bun nv rules --render` from the topic JSON and the fragments, and `--check`
reports a copy that has drifted. Look for the same shape elsewhere before writing a new list by hand:
if a fact can be derived from the files, derive it.

## What a pass never does

- **Never answers an open question.** A fork waiting on a measurement waits for whoever takes the
  measurement. What a pass *may* do is narrow a status field to what is actually open.
- **Never touches the live goal's file, the chain in `docs/goals/README.md`, or the plan's status
  block.** Those are the running loop's state.
- **Never renames a rule.** Constraint 1.
- **Never edits a generated file.** `docs/ground-rules.md` and `docs/rules/<topic>.md` are written
  by `bun nv rules --render`; an edit there is lost on the
  next render, and the file that lost it does not say so. Edit the fragment or the topic JSON.
- **Never changes what a rule decides** — only what its fragment says the current rule is. A change
  of substance is a new goal or a plan entry.

## The judgment half: what the tool cannot see

Read for these directly; there is no check for any of them.

- **Conceptual duplication.** Textual duplication is rare in this corpus. What recurs instead is one
  *rule* explained in two places, usually because one fragment restates a neighbour's table "because
  this is the boundary a reader lands on first". Decide which is the home, and make the other a `rule:`
  citation.
- **A Declined list that contradicts its own rule.** A fragment whose body was edited can still decline
  the thing it now does. Read the Declined list against the body whenever the body changed.
- **A doc-vs-code disagreement.** If the code shipped something a rule still calls designed, or carries
  a type a rule describes and the compiler never had, say so and fix it. Flag the call to the user
  rather than making it silently.
- **A claim with no home.** "The tenth deliberate divergence" was a fact nobody owned. When a pass
  finds prose counting or summarising across rules, that is the signal for a register generated from
  the rules, not for better prose.

## Finishing

1. `bun nv rules --check` and `bun nv links` both clean.
   There is no build to break — this is a docs pass, and a broken or mis-cased cross-link is the only
   mechanical risk.
2. If the pass changed the shape of a rule fragment, update [conventions.md](conventions.md)
   § *A rule fragment* in the same commit — that section is the shape's only copy.
3. Commit with line counts before and after in the message.
4. Follow [AGENTS.md](../../AGENTS.md)'s session workflow: overwrite the live goal's handoff — unless
   a loop is mid-goal, in which case say so in chat and leave that handoff to the loop.
