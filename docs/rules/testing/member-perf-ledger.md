A member's record is four counts and one clock, taken in one sweep over one bench program. The
counts — statements executed, calls made, allocations, bytes, each per operation — are what the
program did, read off `rule:testing/debug-probes`'s sites in `rule:testing/bench-counters`'s
counting mode and off the allocator's own totals. They are the same on every machine and every day
for the same commit, so the report diffs each against the previous record wherever that was taken.
The clock — `ns/op`, the fastest of the reps, with the median beside it — is wall clock with the
empty program's start-up floor subtracted; `units` is that figure divided by a fixed calibration
program measured in the same sweep. Every record carries a machine fingerprint and the report
**refuses to print a clock delta across two of them** — `units` divides out clock speed well enough
to see a threefold regression and not well enough to claim five percent, and a clock delta inside
the spread the median shows is the machine rather than the code. There is no column for another
engine.

**A bench declares what it expects, and its first run is judged against that rather than against
history.** `// bench: allocations 0` and its siblings name a count per operation; `// bench:
complexity constant` with a `<name>.scale.nvs` sibling names how the clock may grow with the input,
checked as a ratio inside one run — the one wall-clock comparison that holds on any machine. A bench
that misses what it declared is a failing proof: no record is written, and
`rule:testing/a-failing-proof-is-fixed-or-recorded` names the two answers.

**What the counts cannot see is accepted.** A `Core` member is one helper call however much it does
inside, so a member that got slower without allocating is invisible to every count and visible only
to the clock on the machine that took it and to the scaling ratio. The ledger's job is to find
regressions and to hand a person with a profiler a shortlist — the report's *Candidates* — and never
to certify that a member is as fast as it could be.

**A figure is re-measured only when the implementing file's code changes.** A record's `impl_hash` is
`bun nv proofs --impl-hash` of that file: the tokens the compiler reads, without comments, layout,
test code or reference cards and the links to them — the `card` tier `tools/nv-scan` computes.
A reference chapter that implements a language feature is hashed as its text. That
currency rule is what makes a roster of hundreds affordable, and the code rather than the commit is
what lets a session measure before the wrap commits the tests it spliced into that same file. A
comment, a card or a reformat cannot change what the binary does, so none of them stales a figure.
The granularity is still the file rather than the member, which is conservative in the only safe
direction: the cost of being wrong is one command rather than a wrong number.

The gate accepts a record from any machine and only the report's clock columns insist on this one's,
so a fresh clone owes nothing it already has a current record for. Nothing here gates a build. A
regression is a row with a delta on it.
