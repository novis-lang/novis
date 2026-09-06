`#[Bench]` reports counted semantic work — statements executed, calls made, allocations, bytes
attributed, GC cycles — read off the same probe sites coverage and tracing use, in a counting mode
beside their timing one (`rule:testing/debug-probes`). Those numbers are **bit-identical across
machines, operating systems and architectures**, because they count what the program did rather than
what a CPU did. `bytes` costs no new instrument: memory is already attributable to a request under
an enforceable cap, and this reads that accounting.

CI may gate on the exact rows. It may not gate on wall-clock, which is printed beside them and
labelled advisory on every run.

The counters are comparable across machines and **not** across releases: an optimising tier
eliminates work, so one program's statement count falls between them. "Did my algorithm improve" is
this rule's question; "did Novis get faster" is `rule:testing/perf-two-mechanisms`'s, and the two do
not overlap. There is no per-member cost table and there will not be one — a hand-written claim
about what a `Core` member costs is a number with no guard test.
