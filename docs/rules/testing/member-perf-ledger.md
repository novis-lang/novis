A member's figure is Novis measured against Novis, and there is no column for another engine.
`ns/op` is wall clock with the empty program's start-up floor subtracted; `units` is that figure
divided by a fixed calibration program measured in the same sweep. Every record carries a machine
fingerprint and the report **refuses to print a delta across two of them** — `units` divides out
clock speed well enough to see a threefold regression and not well enough to claim five percent.

The gate accepts a record from any machine and only the report insists on this one's, so a fresh
clone owes nothing it already has a current record for.

**A figure is re-measured only when the commit that last touched its implementing file changes.**
That currency rule is what makes a roster of hundreds affordable. The granularity is the file rather
than the member, which is conservative in the only safe direction: a comment-only edit stales its
file's figures, and the cost of being wrong is one command rather than a wrong number.

Nothing here gates a build. A regression is a row with a delta on it.
