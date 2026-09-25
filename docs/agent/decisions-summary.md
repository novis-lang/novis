# The decision summary pass

How to bring `docs/decisions.toml` — the plain-language summary of every decision
Novis has taken — back level with the decision records. Fire it the way
[doc-cleanup.md](doc-cleanup.md) is fired: the **user** asks for it, or `python
tools/decisions.py --check` has accumulated findings between passes.

**Read this, then start with the tool.** It holds the mechanism, the group list and every rule it can
check by itself. This file holds the one thing it cannot: how to write the prose.

## What this artifact is for, and who reads it

The decision records are written for the agent that has to *change* a rule. They live under
`docs/decisions/`, frozen on acceptance, each reasoning at full length about the rules its `changes:`
block names; what is currently true is the rulebook under `docs/rules/`, and a record is reached
through a rule's `because`. That is the right shape for the corpus — but a person who wants to know
what Novis decided cannot read it, and never will.

The summary is the other half. Its reader is a developer sizing up the language, with no history
here and no patience for ours. They get one paragraph per decision, grouped by what it is about, and
they should be able to stop reading at any point without having been left mid-thought.

**They are reading it on the website**, where it is the page that answers "what is this language,
actually." The repository copy is the same text; the fact that a decision record exists behind each
entry is the renderer's business, not the prose's.

## The loop

```
python tools/decisions.py --check          # what the summary owes: missing, stale, malformed
python tools/decisions.py --work --limit 20   # the work order, with the raw material inlined
                                              #   --group syntax narrows it to one group
                                              #   --limit 0 prints all of it
                              ... write the entries ...
python tools/decisions.py --apply <file>   # merge, validate, render — all of it or none of it
```

`--work` prints a fillable block per decision owed, carrying that decision's title, its `In short`
summary and the rules its `changes:` block created and modified. **That is the whole input.** Do not
open the record itself unless the work order genuinely does not say what was decided — the material it
prints is what a summary is written from, and opening 20 records instead is how this pass stops fitting
in a session.

**An entry stamps a digest of the material it was written from**, and `tools/decisions.py`'s module
doc is the home of exactly what that hashes: the record's frozen title — `# ADR NNNN — …` — and its
`changes:` block, the rule ids the decision created and modified. When either moves, `--check`
reports the entry stale. A stale entry is a *re-check*, not a rewrite: one whose decision still means
the same thing is re-stamped as it stands, under the rule at the end of this file.

Write every entry into one file and apply it once. `--apply` refuses the whole file if any entry
breaks a rule, so a refusal costs you nothing but the fix, and there is never a half-applied batch.

**Stop when `--check` is clean, or when the session's context budget says stop** — whichever comes
first. The budget is the one in [AGENTS.md](../../AGENTS.md) § *Session workflow*; that paragraph is
its only home and this pass does not get its own number. The work is resumable by construction: an
entry already written and current is never looked at again, so the next session picks up exactly
where this one stopped, with no state to hand over beyond the tool's own answer.

## Writing one entry

A headline that reads as a claim, then two to four sentences. That is the whole form.

**Say what is true now.** Never that it changed, never what it used to be, never that one decision
adjusted another. The records carry that history and are welcome to it. An entry that says "this
was later relaxed" has spent a third of its length on the reader's least interesting question.

**Write for someone who knows PHP and not us.** Their words, not ours: "sandbox" not "isolate
arena", "checked while building" not "at compile time in the type pass", "stops the build" not
"emits a hard diagnostic". The tool blocks a short list of words that always fail this test; the list
is not the standard, it is the floor.

**Lead with the consequence, not the mechanism.** "Untrusted input has its own type, and cannot
reach a database or a page" is the decision. How the marking propagates is the second sentence, if
it fits, and the record's if it does not.

**One concrete example beats a sentence of definition**, when the decision is about spelling.
`if ($rows)` earns its space; a description of the truthy table does not.

**Do not sell.** The reader is evaluating, and evaluating people can smell a pitch. Several of these
decisions cost something real — say so plainly where it is true, in the same voice as everything
else. An entry that reads as marketing copy makes the forty around it less believable.

**Pick the group by what the reader is looking for**, not by which part of the compiler owns it.
`python tools/decisions.py --groups` prints the list with what belongs in each. `internal` is for
decisions about building Novis rather than using it — real decisions, kept for the record, in a group
a reader can skip whole.

**Leave `pin` alone unless you are placing the entry that frames a group.** It floats one entry above
decision order, for the case where what a group is *about* would otherwise sit in the middle of it.
One per group, and the tool refuses a second — a group that needs two framing entries is a group
that has been drawn wrong.

## What the tool decides, so you do not have to

`--check` enforces: no cross-reference of any kind in the prose, no decision cited by number,
headline and body length, the jargon floor, and a group that exists. It stamps the digest that makes
the next pass cheap. It writes both generated files.

You decide: which group, what the decision actually means to somebody using the language, and
whether the paragraph is worth a stranger's time.

## Two things never to do

**Never edit `docs/decisions.md` or `website/src/data/decisions.json`.** Both are generated; the next
`--render` silently discards whatever you put there. `docs/decisions.toml` is the only home.

**Never reword an entry whose digest still matches.** Nothing decided has changed, so a rewrite is
churn in the one file that is supposed to be stable — and it costs the budget that the decisions
still owed are waiting on. Rewrite an entry when its decision moved, or when it is wrong.
