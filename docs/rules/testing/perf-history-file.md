A data point is one JSON object appended to `docs/perf/history.ndjson` — commit, date, workload,
instruction count, wall clock, PHP ratio. It is committed to the repository and diffable, with no
dashboard service and no database behind it; a renderer turns it into a trend chart on demand, and
nothing requires that renderer to exist before the data does.

The determinism the instruction count is chosen for is measured of a bare example binary, not of a
whole compile-and-run: two consecutive runs of a real workload agree to about six significant
figures, because the process reads a file and compiles it before any of the workload runs. **The
lower of the two is what gets recorded.** A trend line reads perfectly well at that resolution; a
guard asserting equality between two runs would not, so no such guard exists.
