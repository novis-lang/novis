`Core\Debug` starts and stops each probe from inside a request — coverage with an optional branch
half, a trace to a named sink, a profile to another — and reads coverage back as a path-to-line-to-
count map. `nvs test` and `nvs run` grow flags that wrap a whole run in those calls with no source
change at all, which is the ergonomics CI uses day to day.

What comes out is what existing tooling already reads: Clover and lcov for coverage, Callgrind for
the profile. The trace is newline-delimited JSON, because no third-party tooling widely consumes any
trace format either.

Line and branch counters are bounded by the code a request touches, not by how often a loop
iterates, so they live in the request's arena and are dropped with it. Trace and profile data are
call-count-proportional, so they **stream to a sink** named when they start rather than accumulating
in memory — nothing to bound with a limit this rule would otherwise have to invent.

A spawned isolate starts from its parent's current flag word and may only narrow it. Its own data
never merges into the parent's live state; it comes back as fields on the result, by the same
copy-out rule a value, an error and a usage figure already use.
