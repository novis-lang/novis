`Core\Debug` starts and stops each probe from inside a request — coverage with an optional branch
half, a trace to a named sink, a profile to another — and reads coverage back as a path-to-line-to-
count map. `nvs test` and `nvs run` grow flags that wrap a whole run in those calls with no source
change at all, which is the ergonomics CI uses day to day.

What comes out is what existing tooling already reads: Clover and lcov for coverage, Callgrind for
the profile. The trace is newline-delimited JSON, because no third-party tooling widely consumes any
trace format either. A coverage report opens in the CI dashboard a project already has configured,
and a profile opens in KCachegrind or Webgrind unmodified, so there is no client to write. The price
is three exporters to keep correct instead of one, and a low barrier to entry is worth it.

The profiler is deterministic: it times every call and attributes exact self and inclusive time per
function. A sampling profiler answers a different question, a low-overhead view affordable in
production, and the two stay separate tools.

- **Memory:** line and branch counters are bounded by the code a request touches, not by how often a
  loop iterates, so they live in the request's arena and are dropped with it. Trace and profile data
  are call-count-proportional, so they **stream to a sink** named when they start and hold nothing in
  memory, which leaves no buffer to bound with a new limit.

A spawned isolate starts from its parent's current flag word and may only narrow it. Its own data
never merges into the parent's live state; it comes back as fields on the result, by the same
copy-out rule a value, an error and a usage figure already use.

**Not on disk.** The probe bits exist (`crates/nvs-runtime/src/ctx/mod.rs:287`), and `nvs test` writes
lcov, Clover and Cobertura coverage (`crates/nvs-cli/src/main.rs:617`,
`crates/nvs-cli/src/coverage.rs`). `Core\Debug` has no start, stop or coverage read-back
(`crates/nvs-stdlib/src/debug.rs:117`), there is no Callgrind exporter and no streamed trace sink, `nvs
run` has no probe flags, and a spawned isolate's probe data does not come back on its result.

**Declined:**
- **A Novis-native coverage and profile format with a converter.** Both already have dominant
  consumers, and a conversion step is friction for every project that uses them.
- **Xdebug's own trace-file format.** Nothing beyond Xdebug's own tooling reads it widely, so
  matching it would buy no consumer.
- **Merging a child's coverage or trace into the parent's live state.** That needs mutable state
  crossing the arena boundary isolation exists to keep (`rule:security/isolate-shares-nothing`),
  and returning it on the result costs nothing new.
