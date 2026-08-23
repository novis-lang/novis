# Documentation cleanup prompt

Reusable prompt for periodically compacting MWL's docs (ADRs, `docs/adr/README.md`,
`docs/implementation-plan.md`, `docs/spec/`, `AGENTS.md`) as the ADR count grows. Paste this whole
file as the prompt when you want another pass. Update it in place if a future pass finds a rule that
needs adjusting — don't let it drift out of sync with how cleanup is actually done.

## When this pass is due

**Not on a budget signal — there is no budget signal.** Nothing in this repository measures a doc against
a size, by design: see AGENTS.md § *Length targets, and why nothing enforces them*. Doc length never
triggers this pass.

Run this pass when *you* judge the docs have drifted — rationale piled up, a decision superseded in three
places, an ADR nobody can find. There is no schedule and no automatic signal; the user fires it by hand.

One thing is never a target here: the ADR index in `docs/adr/README.md` grows a row per decision and is
*supposed* to, as does the `## Where to look` routing table above it. Neither is trimmable.

## The ask

We've done a lot of work on the documentation. Before proceeding with the next milestone prompt, go
over everything and clean it up:

- Cleanup, compact, deduplicate, streamline everything so the project is in a cleaner state and new
  sessions/developers can pick it up easily.
- Combine/Merge ADRs where it makes sense.
- Remove any decision or information that is now irrelevant and doesn't benefit the project.
- Don't keep every single historical decision in the live docs just because it happened.
- Where a choice deliberately diverges from PHP, don't excessively justify it — state that it was a
  definitive decision made during brainstorming and move on. Keep the "why we're not doing it PHP's
  way" reasoning to a bare minimum everywhere it appears.
- If we made specific decisions that have a amendment that overrides the previous decision, collapse the decisions so not all the history of an idea flow will be kept in the docs. We have git versioning, everything ever existed is still in the git history, we dont need it in the docs. At the current stage, we can accept to override the "Accepted" areas in an ADR, because we are still in early prototyping phase.

## Standing rules for this pass (revised 2026-08-23 — the 2026-08-21 set is superseded)

**Fold, never overlay.** This project used to amend a decision by adding prose to the *amended* ADR's
metadata block describing what a later one changed. That is retired: an ADR's body always states the
current rule, `Amended by:` is a bare list of numbers, and the amending ADR's `Amends:` is one clause.
`docs/adr/README.md` § *Adding a decision* holds the rule. When you find a body that still describes a
superseded rule, fix the body — that is the whole job, not a merge question.

**Merging is allowed, and so is rewriting an Accepted `## Decision`.** The project is in prototyping; git
holds the history. Fold an ADR away when it has **zero surviving unique content** once the fold is
applied — 0032 was exactly that, its entire content being "0029 § 1 is revoked" — and retire its number
rather than renumbering anything. Do **not** fold one whose rule is genuinely its own even when it reads
as coupled: 0027 and 0031 stayed separate because 0027 owns which values satisfy `callable` and 0031 owns
the literal, and 0027 is referenced from sixteen code sites by section number. Check `grep -rn "00NN"
crates/` before deciding: a file referenced from code costs more to retire than it saves.

**A doc-vs-code disagreement outranks a status field.** If the code has shipped something an ADR still
calls Proposed, or carries a type the ADR describes and the compiler has never had, say so and fix it —
that is exactly what this pass is for. Flag the call to the user rather than making it silently.

**Trim depth: cut rationale to bullet lines.** Every ADR already follows metadata block → `## Decision`
(within ~60 lines) → optionally `## Context` / `## Investigation` / `## Alternatives rejected` →
`## Consequences`. `AGENTS.md` already tells readers the sections after `## Decision` are skippable
unless they intend to overturn the decision — so trim those sections hard:
- Keep the metadata block's *fields* (Status, Date, Scope, Amends/Amended-by, Validated by — load-bearing
  for the README index), but not its prose: an `Amended by:` is numbers, a `Relates to:` longer than
  three lines is numbers, and an `Amends:` is one clause per target.
- Change `## Decision` and `## Consequences` only to make them state the current rule; otherwise tighten
  padding and leave the meaning alone.
- Compress `## Context` / `## Investigation` / `## Alternatives rejected` into short bullet lists: one
  bullet per alternative considered (name + one-phrase reason rejected), one or two bullets for "why we
  deviated from PHP" where applicable. Target roughly 5-15 lines per section, not 40-100 lines of prose.
  Delete extended comparison tables, multi-paragraph reasoning chains, and prior-art surveys ("how Go/
  Rust/Java/Python handle this") down to at most a one-line callout if it's genuinely load-bearing,
  otherwise cut it entirely.
- If a section has nothing left worth keeping as a bullet, delete the heading too rather than leaving
  an empty stub.
- Never touch diagnostic names/codes or cross-reference links while trimming, and never change what a
  `## Decision` *decides* — only what it *says the current rule is*.

**Scope: everything.** ADRs, `docs/adr/README.md`, `docs/implementation-plan.md`, `docs/spec/`, and
`AGENTS.md` are all in scope for this pass — not just the ADRs. Apply the same "one fact, one home, no
padding" standard everywhere; if a doc restates something the ADR already owns, delete the restatement
and link instead.

**Don't resolve an open decision.** A cleanup pass never answers a question an ADR left open — ADR 0009
§ 2's granularity fork waits on its measurement, not on you. What a pass *may* do is narrow the status
field to what is actually open: 0009 moved to Accepted here because §§ 1, 3 and 4 had shipped and only
§ 2 was in question, and the index had been implying the whole `bytes` type was undecided.

## How to run this pass

1. Read `AGENTS.md`, run `python tools/brief.py`, and read `docs/adr/README.md` to get the current shape
   of the doc set (ADR count, statuses, what's Accepted vs Proposed).
2. Survey for merge candidates and stale/irrelevant content before editing anything — read-only pass,
   report findings, get human approval on any proposed merge before acting on it.
3. Execute the trim across every ADR (batch it — e.g. one subagent per handful of files — to keep this
   from consuming the whole context window; review a sample of the diffs afterward for consistency and
   to make sure `## Decision` sections were left untouched).
4. Re-check `docs/adr/README.md`, `docs/implementation-plan.md`, `docs/spec/`, and `AGENTS.md` for
   anything the trim pass should have caught but is out of ADR scope.
5. Run `python tools/check-links.py` — this is a docs-only pass, so the main risk is a broken or
   mis-cased cross-link, not a build break, and that is exactly what it reports. There is still no
   markdown linter here, and no doc-size check of any kind.
6. Commit with a message describing the trim (line counts before/after are a good thing to mention).
7. Follow `AGENTS.md`'s "Keep work small, commit your work" section: write the next-session prompt to
   `docs/agent/handoff.md`, replacing its prior content, and show it to the user.

## Open questions to ask if unclear next time

- Is the fold-don't-overlay convention still what `docs/adr/README.md` § *Adding a decision* says? That
  file is authoritative; this one restates it only so a pass run from here has the rule in hand.
- **Not yet done, and the largest remaining target:** the `## Context` / `## Alternatives rejected` /
  `## Revisiting` sections are still ~27% of the ADR corpus (~260 KB) at full prose length. The trim
  depth above is written for exactly that pass; the 2026-08-23 run spent its budget on contradictions and
  duplication instead and left this untouched.
- Has the ADR count grown enough that `AGENTS.md`'s "Where to look" table itself needs restructuring
  (e.g. grouping several ADRs under one table row) rather than just trimming individual files?
- Is `docs/spec/` still a stub, or has it grown enough to need its own cleanup pass with different
  rules than the ADRs?
