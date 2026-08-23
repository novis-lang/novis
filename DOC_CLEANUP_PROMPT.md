# Documentation cleanup prompt

Reusable prompt for periodically compacting MWL's docs (ADRs, `docs/adr/README.md`,
`docs/implementation-plan.md`, `docs/spec/`, `CLAUDE.md`) as the ADR count grows. Paste this whole
file as the prompt when you want another pass. Update it in place if a future pass finds a rule that
needs adjusting — don't let it drift out of sync with how cleanup is actually done.

## When this pass is due

**Not on a budget signal — that is no longer what this pass is for.** `python .claude/brief.py --check`
enforces per-entity caps (one status field, one milestone heading, one ADR index decision cell) and its
section budgets are derived from entity counts, so doc growth cannot trip it. A `--check` failure names a
single over-long line with the bytes to cut: fix that line where it lives and move on. It is a lint hit,
not a trim-pass trigger, and running this whole pass in response to one is wasted effort.

Run this pass when *you* judge the docs have drifted — rationale piled up, a decision superseded in three
places, an ADR nobody can find. There is no schedule and no automatic signal.

One thing is never a target here: the ADR index in `docs/adr/README.md` grows a row per decision and is
*supposed* to. It is not trimmable. Its only constraint is that each decision cell stays one sentence,
which `--check` already enforces.

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

## Standing rules for this pass (decided 2026-08-21, keep applying unless told otherwise)

**Merge policy: default to NOT merging.** This project amends a decision via a *new* ADR with
`Amends`/`Amended by` cross-links rather than rewriting an existing `Accepted` ADR's `## Decision`
section (see [feedback: ADR amend, don't rewrite] in memory, and `docs/adr/README.md`'s own "Adding a
decision" section). Two ADRs reading as "one decision split across files" — even tightly coupled ones
like 0029+0030 or 0027+0028 — is that convention working as intended, not fragmentation to fix. Only
propose a merge when both halves are genuinely still `Proposed`/unreleased (nothing downstream depends
on the split yet), or when a file has zero surviving unique content after later ADRs superseded it
entirely. Always list merge candidates for human approval before touching files — never merge freely.

**Trim depth: cut rationale to bullet lines.** Every ADR already follows metadata block → `## Decision`
(within ~60 lines) → optionally `## Context` / `## Investigation` / `## Alternatives rejected` →
`## Consequences`. `CLAUDE.md` already tells readers the sections after `## Decision` are skippable
unless they intend to overturn the decision — so trim those sections hard:
- Keep the metadata block exactly as-is (Status, Date, Amends/Amended-by links — these are load-bearing
  for the README index and CLAUDE.md's routing table).
- Keep `## Decision` and `## Consequences` untouched in meaning; only tighten prose that's clearly
  padding.
- Compress `## Context` / `## Investigation` / `## Alternatives rejected` into short bullet lists: one
  bullet per alternative considered (name + one-phrase reason rejected), one or two bullets for "why we
  deviated from PHP" where applicable. Target roughly 5-15 lines per section, not 40-100 lines of prose.
  Delete extended comparison tables, multi-paragraph reasoning chains, and prior-art surveys ("how Go/
  Rust/Java/Python handle this") down to at most a one-line callout if it's genuinely load-bearing,
  otherwise cut it entirely.
- If a section has nothing left worth keeping as a bullet, delete the heading too rather than leaving
  an empty stub.
- Never touch `## Decision` content, diagnostic names/codes, or cross-reference links while trimming.

**Scope: everything.** ADRs, `docs/adr/README.md`, `docs/implementation-plan.md`, `docs/spec/`, and
`CLAUDE.md` are all in scope for this pass — not just the ADRs. Apply the same "one fact, one home, no
padding" standard everywhere; if a doc restates something the ADR already owns, delete the restatement
and link instead.

**Proposed ADRs stay Proposed.** Don't resolve an open decision (e.g. ADR 0009's `string`/`bytes`
question) as part of a cleanup pass — trim its prose like any other ADR, but leave its status and the
open question itself alone. Cleanup is not the moment to make new calls.

## How to run this pass

1. Read `CLAUDE.md`, run `python .claude/brief.py`, and read `docs/adr/README.md` to get the current shape
   of the doc set (ADR count, statuses, what's Accepted vs Proposed).
2. Survey for merge candidates and stale/irrelevant content before editing anything — read-only pass,
   report findings, get human approval on any proposed merge before acting on it.
3. Execute the trim across every ADR (batch it — e.g. one subagent per handful of files — to keep this
   from consuming the whole context window; review a sample of the diffs afterward for consistency and
   to make sure `## Decision` sections were left untouched).
4. Re-check `docs/adr/README.md`, `docs/implementation-plan.md`, `docs/spec/`, and `CLAUDE.md` for
   anything the trim pass should have caught but is out of ADR scope.
5. Run `cargo fmt --check` / whatever doc-adjacent checks exist (there's no markdown linter in this
   repo as of this writing) — this is a docs-only pass, so the main risk is a broken cross-link, not a
   build break. Spot check a few `[ADR NNNN](...)` links resolve to real files.
6. Commit with a message describing the trim (line counts before/after are a good thing to mention).
7. Follow `CLAUDE.md`'s "Keep work small, commit your work" section: write the next-session prompt to
   `NEXT_SESSION_PROMPT.md`, replacing its prior content, and show it to the user.

## Open questions to ask if unclear next time

- Has the "amend, don't rewrite" convention changed? (Check `docs/adr/README.md`'s own process section
  and recent git history for ADR-add commits before assuming the answer above still holds.)
- Has the ADR count grown enough that `CLAUDE.md`'s "Where to look" table itself needs restructuring
  (e.g. grouping several ADRs under one table row) rather than just trimming individual files?
- Is `docs/spec/` still a stub, or has it grown enough to need its own cleanup pass with different
  rules than the ADRs?
