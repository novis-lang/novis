# How long a milestone actually takes

The milestone table in [../implementation-plan.md](../implementation-plan.md) carries two figures per row:
the **original estimate**, written for a human team before any code existed, and a **projection in
loop-days**. This file is the one home for how the second is derived. It holds no per-milestone day
counts — those are the table's, and restating them here would give them two homes.

**The conversion is ~23×: one estimated week ≈ 7 hours of loop time.** That is measured, not hoped for.

## What it is measured against

M0 through M3 were estimated at 73 days and took 3.1. `git log` is the source; these are its milestone
boundaries, not a second record to keep in sync:

| Milestone | Estimated | Elapsed | Ratio |
|---|---|---|---|
| M0 | 3 d | 0.3 d | 9× |
| M1 | 3 w | 0.7 d | 30× |
| M2 | 4 w | 1.5 d | 18× |
| M3 | 3 w | 0.5 d | 40× |
| **M0–M3** | **73 d** | **3.1 d** | **23×** |

Two things make the ratio mean what it says. The commit-hour histogram is flat — every hour of the day
carries commits — so the loop runs continuously and **a calendar day is a working day**. And the original
estimates assumed a human week, so the like-for-like figure against 8h/day is nearer 100×; 23× is the
wall-clock one, which is what a schedule cares about.

Re-measure rather than trusting this file's age:

```
git log --format='%ad' --date=format:'%H' | sort | uniq -c     # is the loop still continuous?
git log --format='%ad %h %s' --date=format:'%m-%d %H:%M' --reverse | grep -i 'milestone\|is done'
```

## Why the table does not apply 23× uniformly

The estimates were written for humans, and what slows a human is not what slows this loop. Four classes,
and the multiplier each gets:

- **Spec-driven and self-verifiable — 20–25×.** A written spec, a machine-checkable gate, no external
  system. M4, M4S and M6: a written rule plus a conformance case that pins it is the best case this
  loop has.
- **Integration-bound — 15–18×.** Real servers, real wire protocols, a dependency that has to be stood
  up before it can be debugged. M7, M8's drivers, M9.
- **Human-in-the-loop — 12–15×.** Anything whose acceptance is a person looking at it: M4B's extension,
  M10's editor surfaces.
- **Nondeterministic or wall-clock-bound — 10×.** Races, schedulers, soak runs. M5 is the weakest case
  for an agent in the whole plan, and fuzz and valgrind campaigns cannot be compressed by writing code
  faster.

**Then add 35% for rework**, applied to the total rather than to a row. This is not a hedge; the history
already spent it. M1 was re-opened four times after being reported done, Stage 0's catch-up list reached
22 items from ADRs that landed after the milestone owning them closed, and `rule:expressions/one-equality-operator`'s spelling change had
reached 48 files before item 1 rewrote them. That cost grows with surface area.

M12 gets no projection. It is measurement-bound — the ledger is
[docs/perf/userland-gap.md](../perf/userland-gap.md), and closing a gap there is benchmark iterations, not
code volume.

## What would break the projection

In the order the risk is real:

1. **A design call the loop cannot make.** `Blocking: Nothing` holds only because
   the live goal's § *Standing decisions*, in its prose under `../agent/goals/`,
   pre-authorises every call the loop reaches. Each one that is not pre-authorised converts loop-hours to human-hours at 1:1. This is
   the largest lever, and it is the user's.
2. **Context, which is the binding budget** — AGENTS.md § *Session workflow* step 2 owns the ceiling and
   the number. As the tree grows, a cross-cutting change needs more sessions per unit of work. That is
   why M5 and M10 are decayed above and M6 is not.
3. **Depth costing more than breadth.** Conformance cases landed 324, then 112, then 52 a day. Read that
   as the *unit* shrinking rather than velocity falling — commits per day held steady across the same
   window, and a depth case pins one claim where a breadth case covered a member. But if the decay turns
   out to be intrinsic, M8's 18× is the first figure that slips.
4. **A calendar floor no multiplier touches.** M15 has to stand up a registry: hosting, key management, a
   threat model somebody signs. The code compresses; that does not.

**Nothing enforces any of this.** It is a projection to weigh when scheduling, re-measured from `git log`
when it matters — never a gate, and never something to trim a session against.
