# Handoff

## State

**Goal `gap-zero` is met — every stage that carries a check is green.** Stage 3: `python
tools/playbook.py --check` reads `none -- every path any bullet names still exists`, `python
tools/owners.py --registers` reads 5 registers with the deleted index absent as it requires, and
`python tools/check-links.py` and `python tools/chain.py --check` pass. Stage 5: `python
tools/plan.py --past` reads 11 of 11 past milestones complete and `--check` finds every `Carried by`
cell agreeing with the chain.

**Stage 4 carries no check, by the user's decision of 2026-09-18.** GitHub starts no CI job while the
account's payments fail, so the CI check moved whole to goal `ci-green`, pinned last on the chain. A
session here neither asks `gh` nor reports a `BLOCKED` about CI.

**The three gates a goal meets only at its end are green**: `python tools/verify.py --doc`, `python
tools/owners.py --closes gap-zero` (owns no module-doc gap) and `python tools/playbook.py --closes
gap-zero` (owns no row).

## Next group

**Goal `dossier` — its own seed replaces this file at the switch** — one file set:
`docs/agent/goals/71-dossier.*`.

- [ ] **Take goal `dossier`'s first item from its seed handoff** —
      `docs/agent/goals/71-dossier.handoff.md:1` names the group and the file set it shares; the
      driver installs it when it walks past goal `gap-zero`, so nothing here needs carrying forward.

## Backlog

- Goal `ci-green` needs `main` pushed and a green `ci.yml` run, and waits on the account's billing —
  `docs/implementation-plan.md` § *Blocking*.
- 24 module-doc gaps stand, each deferred to a milestone still ahead — `python tools/owners.py
  --check` is the list, and each item lives in the module doc that owes it.
- 327 playbook bullets carry a live `[until:]` trailer; `docs/agent/playbook.md` grows faster than
  either pruning signal can fall — `python tools/playbook.py --check` § *HOW FAST THIS FILE IS
  GROWING*.
