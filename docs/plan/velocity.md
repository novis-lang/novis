# How long a milestone takes

The milestone table in [../implementation-plan.md](../implementation-plan.md) carries two figures per row:
the **original estimate**, written for a human team, and a **projection in loop-days**. This file is the
one home for how the second is derived from the first. It holds no per-milestone day counts — those are
the table's, and restating them here would give them two homes.

**The conversion is ~23×: one estimated week ≈ 7 hours of loop time.** It is an estimate for the work
that is left, taken from `git log`, and it gates nothing.

The ratio is wall-clock. The loop runs continuously, so **a calendar day is a working day**. Against a
human 8-hour day the like-for-like figure is nearer 100×; 23× is the one a schedule needs.

Re-measure rather than trusting this file:

```
git log --format='%ad' --date=format:'%H' | sort | uniq -c     # is the loop still continuous?
git log --format='%ad %h %s' --date=format:'%m-%d %H:%M' --reverse | grep -i 'milestone\|is done'
```

## Why the table does not apply 23× uniformly

The estimates were written for humans, and what slows a human is not what slows this loop. Four classes,
and the multiplier each gets:

- **Spec-driven and self-verifiable — 20–25×.** A written spec, a machine-checkable gate, no external
  system. A written rule plus a conformance case that pins it is the best case this loop has.
- **Integration-bound — 15–18×.** Real servers, real wire protocols, a dependency that has to be stood
  up before it can be debugged. M15's registry client, M16's scaffold on three platforms.
- **Human-in-the-loop — 12–15×.** Anything whose acceptance is a person looking at it: M10's editor
  surfaces.
- **Nondeterministic or wall-clock-bound — 10×.** Races, schedulers, soak runs. Fuzz and valgrind
  campaigns cannot be compressed by writing code faster.

**Then add 35% for rework**, applied to the total rather than to a row. A decision that lands after the
work it touches reopens that work, and the cost grows with surface area.

M12 gets no projection. It is measurement-bound — the ledger is
[docs/perf/userland-gap.md](../perf/userland-gap.md), and closing a gap there is benchmark iterations, not
code volume.

## What would break the projection

In the order the risk is real:

1. **A design call the loop cannot make.** A goal's § *Standing decisions* pre-authorises the calls its
   stages reach. Each call that is not pre-authorised converts loop-hours to human-hours at 1:1. This is the
   largest lever, and it is the user's.
2. **Context, which is the binding budget** — AGENTS.md § *Session workflow* step 2 owns the ceiling and
   the number. As the tree grows, a cross-cutting change needs more sessions per unit of work, which is
   why M10 is decayed above.
3. **Depth costing more than breadth.** A depth case pins one claim where a breadth case covered a
   member, so cases a day fall while commits a day hold. If that decay turns out to be intrinsic, the
   spec-driven multiplier is the first figure that slips.
4. **A calendar floor no multiplier touches.** M15 has to stand up a registry: hosting, key management, a
   threat model somebody signs. The code compresses; that does not.

**Nothing enforces any of this.** It is a projection to weigh when scheduling, re-measured from `git log`
when it matters — never a gate, and never something to trim a session against.
