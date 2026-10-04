---
milestone: post-parity
position: last
---
# Loop goal 185 — every feature checks its own growth, and a change reruns only the benches it reaches

Goal `performance-pass` found and fixed the growth problems that exist today. This goal makes sure no
new one gets in. Growth becomes part of every feature's perf proof, so a new feature is checked the
moment its bench first runs. A change to existing code reruns the benches that reach that code, and
only as many of them as it takes to prove nothing got worse.

Three things make that cheap enough to run on every change:

- **Small benches.** A bench exists to catch growth, not to load the machine. Its iteration count
  comes down to what its growth check needs, and a budget keeps the bench tree from creeping back up.
- **Valgrind only to find a threshold.** It runs only when the normal ramp shows no clear growth. It
  never runs in `nv verify` and never in an ordinary bench run.
- **Only the benches a change reaches.** A bench gets a footprint, as an example already has, so the
  tooling knows which benches a change reaches. Once the first of them show no degradation, the
  others that reach only the same already-proven code are skipped.

## Why here

**Behind goal `performance-pass`, by the user's decision of 2026-10-01, and never before it.** The
growth proof is switched on only once every existing bench passes the ramp. Goal `performance-pass`
fixed every bench that did not, so switching the proof on here makes nothing owed. A goal before this
one never owed it.

It reuses goal `performance-pass`'s tool. `bun nv scaling` and its doubling ramp already exist and
are proven on the whole bench tree, so this goal moves the ramp into the perf proof rather than
writing a second one.

**What this spends:** a recording run per bench on the `covws` debug build when its footprint is
taken, the same price an example already pays (`tools/nv/proofs/select.ts`). It is paid once per bench
definition and per change that reaches the bench, not on every run.

## Stage 0 — the catch-up

None.

## Stage 1 — the floor

Nothing is carried. The driver deletes goal `performance-pass` when this goal starts. The suites, the
`.nvst` trees and `nv verify` are the floor, and this goal's Stage 6 checks that the feature proofs
owe nothing.

## Stage 2 — growth is a feature proof

**Does:** Makes every feature's perf proof run the growth ramp over its bench, with valgrind only as the fallback for an unclear result.

- **The record.** One new decision record, written first. It modifies `rule:testing/feature-proofs`:
  **Perf** is one measured figure *and its growth*. The fragment is rewritten whole.
  [AGENTS.md](../../../AGENTS.md) rule 9 and [conventions.md](../conventions.md) § *Feature proofs*
  say "one bench, with its growth" where they say "one bench" today.
- **What the proof asks.** The perf proof runs the ramp of `tools/nv/cmd/scaling.ts` over the
  feature's bench. The per-operation counts must stay the same as the batches double. Where the work
  depends on input size, the cost must grow no faster than the bench's declared complexity, under the
  bounds in that tool's module doc.
- **`// bench: complexity` is required on every bench.** It may be `constant`, `linear`, `nlogn`, or a
  named class an algorithm has. `quadratic` is never accepted. `constant` needs only the iteration
  ramp. Any other class also needs a size ramp: the `.scale.nvs` sibling, which takes `start` and `max`
  as a ladder does instead of its single `// bench: scale K`. A bench with no declaration fails its
  proof, so a new feature cannot skip the size check by leaving the line out. The benches that lack it
  get it in this stage, from what goal `performance-pass`'s ramp measured. Subagents write them in
  batches, by area.
- **Where it runs.** `tools/nv/proofs/perf.ts` replaces its one-ratio scale test (`COMPLEXITY_RE` and
  the `SCALE_TOLERANCE` check) with a call to the ramp, so `bun nv proofs --record-perf` and `bun nv
  proofs --id '<feature>'` judge growth with no extra command. A failure is a failing proof, and
  `rule:testing/a-failing-proof-is-fixed-or-recorded` names its two answers.

**The noise problem, and valgrind, by the user's decision of 2026-10-01.**

- The ramp decides on the counts, which are the same on every run and every machine. The clock is
  read and reported, and it never decides.
- **Valgrind runs only when the ramp shows no clear growth.** That happens when the counts stay flat
  while the work clearly happens inside a Rust member, and the clock is too noisy to read before the
  ramp reaches its ceiling. Only then does the proof run the same batches again under `valgrind
  --tool=callgrind` in WSL, without being asked. Callgrind's instruction count is the same on every
  run, so it decides the way the counts do.
- **Valgrind only finds the threshold.** It finds the batch at which the growth is clear. The bench's
  `// bench: iterations N` is set from that, and the bench says beside it that callgrind measured it.
  Every later run of the proof uses N with the counts and the clock, and valgrind does not run again.
  It runs again only when the bench's definition changes, or when the normal ramp at N shows no clear
  growth again.
- **Valgrind never runs in `nv verify`, and never in an ordinary bench run.** A test in
  `tools/nv/test/proof-growth.test.ts` fails if the plan `nv verify` runs could start valgrind over a
  bench.
- A bench that is still unclear after callgrind is reported, not judged, and Stage 6 lists it.

## Stage 3 — small benches, and a budget

**Does:** Lowers every bench's iteration count to what its growth check needs, and keeps the bench tree from growing back.

- **The iteration counts come down, by the user's decision of 2026-10-01.** Every bench's
  `// bench: iterations N` is lowered to about twice the batch at which its ramp found a clear
  pattern. N is then also the ramp's cap. `bun nv scaling --iterations --sized` fails while a bench's
  N is larger than that. Lowering N moves no count per operation, and every clock figure is recorded
  again in the same slice. `benches/members/README.md`'s rule "Size it to run in well under a
  second" becomes "the smallest count that shows the growth".

Three rules keep the tree from growing over time, by the user's decision of 2026-10-01. A noisy
threshold must never be able to push iteration counts up.

- **Only exact figures set a size.** The ramp stops on the counts, or on callgrind's instruction count
  when Stage 2's fallback ran. The clock never sets N, so N is the same every time it is computed.
- **The ceiling is fixed.** The ramp's largest batch is a constant in `tools/nv/cmd/scaling.ts`. No
  session raises it, and no check is made green by raising it. A bench that reaches the ceiling
  without a clear pattern keeps its N and is listed in Stage 6. Raising the ceiling is the user's call.
- **The tree has a budget.** `bun nv scaling --budget` adds up the counted work of every bench at its
  N. The total is stored in `docs/perf/bench-budget.json` and may only go down, with two exceptions.
  A new bench adds its own share. A raised N on an existing bench needs a reason written beside its
  `// bench: iterations` line. Growth in the budget is then an exact number in a diff.

## Stage 4 — a change reruns the benches it reaches

**Does:** Gives every bench a footprint, so the tooling knows which benches a change reaches and reruns their perf proof.

Today a perf figure goes stale only when its feature's implementing file changes
(`rule:testing/member-perf-ledger`). A change to the runtime's array code makes `Core\Arr::sort`
slower and leaves its figure current. This stage closes that hole with the observed selection that
examples and attacks already use (`rule:tooling/a-check-runs-only-when-the-change-reaches-its-footprint`).

- **The record.** One new decision record. It modifies `rule:testing/member-perf-ledger`: a figure is
  current while no item its bench's footprint holds has moved. The implementing file's hash stops
  being the currency.
- **A `bench` atom.** `tools/nv/select/atoms.ts` gains the kind `bench`: one bench under
  `benches/members/`, defined by the bench, its `.scale.nvs` sibling, its `.in` and `.nvsr`. Its
  footprint comes from one recording run on the `covws` debug build, as `tools/nv/proofs/select.ts`
  takes an example's. The recording run uses the smallest batch, because a footprint does not depend
  on how many times a line runs.
- **The tooling asks it.** `bun nv affected` names the benches a change reaches. `--run` reruns
  their perf proof at their N: counts and the ramp, and valgrind only under Stage 2's fallback. The
  driver's sweep reruns them the same way. `nv verify` does not: it marks them owed, as it marks the
  examples a change reaches.

## Stage 5 — skip what is already proven

**Does:** Stops rerunning benches once the code a change touched is proven by the first benches that reach it.

A change to shared code, such as the array type in `crates/nvs-runtime`, reaches hundreds of benches.
Rerunning all of them proves the same thing many times. So the rerun is planned, by the user's
decision of 2026-10-01:

- **Order.** The benches a change reaches are ordered so the first ones reach the most changed items.
  The order is deterministic, so the same change always picks the same benches.
- **Proven.** A changed item is proven when two benches that execute it show no degradation. When
  fewer than two benches reach it, every bench that reaches it runs. No degradation means the
  per-operation counts did not rise and the growth stays within its declared complexity.
- **Skipped.** A bench whose changed items are all proven is skipped. Its figure is carried to the new
  tree, and the ledger names the benches that proved it, so the gate does not owe it.
- **Degradation runs everything.** When a bench shows a degradation, every bench that reaches the same
  changed items runs, with no skipping, so the report shows the whole extent of the problem.
- **The report.** `bun nv affected` says how many benches the change reached, how many ran and how
  many were skipped, and which benches proved the skipped ones.

## Stage 6 — what the user reads

**Does:** Adds a plain section on the growth proof to `docs/perf/performance-pass.md`, and leaves the roster owing nothing.

A section `## The growth proof` goes into the summary goal `performance-pass` wrote, in the same plain
voice. It says what the proof checks, in a few sentences. It says how much smaller the bench tree's
work is after Stage 3, as a before and after. It says how many benches an ordinary change reruns and
how many it skips, from a real change the session made. It lists every bench that needed callgrind or
reached the ceiling without a clear pattern, one line each, or says "none".

## Standing decisions

- **The growth proof starts here, never earlier, by the user's decision of 2026-10-01.** The record
  says so.
- **Valgrind is a fallback, by the user's decision of 2026-10-01.** It runs only when the normal ramp
  shows no clear growth, and only to find the threshold. It is never part of `nv verify` or an
  ordinary bench run.
- **Counts decide, the clock reports.** No clock figure fails a proof, sets an iteration count or
  decides that a bench is proven.
- **Skipping never hides a degradation.** A degradation turns skipping off for every bench that shares
  the changed items.
- **Nothing found stops the run.** A bench the proof cannot judge is listed in Stage 6's section. No
  session reports `BLOCKED` for it.
- **Subagents.** Stage 2's backfill of `// bench: complexity` runs one subagent per area. Each stays
  under 250k of context, writes any scratch file under `.agent-tmp/`, and deletes it before it reports.
- **ADR slots:** two new records, Stage 2's for the growth proof and Stage 4's for the ledger's
  currency, and no other number.
