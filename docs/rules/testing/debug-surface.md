`Core\Debug` starts and stops each probe from inside a request — coverage with an optional branch
half, a trace to a named sink, a profile to another — and reads coverage back as a path-to-line-to-
count map. `nvs test` and `nvs run` grow flags that wrap a whole run in those calls with no source
change at all, which is the ergonomics CI uses day to day.

**The members are one start and stop pair per probe and one read-back**, so no member takes a mode
(`rule:core-api/no-mode-strings`):

```text
Core\Debug::startCoverage(bool $branches = false): void
Core\Debug::stopCoverage(): void
Core\Debug::coverage(): array<array<int>>
Core\Debug::startTrace(string $path): void
Core\Debug::stopTrace(): void
Core\Debug::startProfile(string $path): void
Core\Debug::stopProfile(): void
```

- **A start narrows within `[debug] mode` and never widens it** (`rule:testing/debug-mode-directive`).
  The ceiling is the mode plus every probe the host itself turned on, such as `nvs test`'s coverage
  flags. A start whose probe is outside it throws a `RuntimeError` naming the line to add, and turns
  nothing on. `$branches` needs `branch` as well as `coverage`. A stop always succeeds, since it only
  narrows, and it also turns off a probe the mode armed. A child context's ceiling is its parent's
  flag word at the spawn.
- **A trace or profile path is admitted under its own grant**, `debug.trace` or `debug.profile`
  (`rule:testing/debug-probes`), through the same path door `fs.write` uses, and asked before the mode.
  A second start while one sink is named throws a `LogicError`, so one probe writes one file at a time.
- **`startProfile` writes speedscope**, the timeline format; Callgrind comes from the CLI flag. One
  format per member keeps the surface small.
- **`coverage()` is path to line to count**, an `array` keyed by file path whose values are arrays
  keyed by line number. A line's count is the largest count of a statement on it, the rule the
  `nvs test` reports use, and a line that did not run is not listed, so the answer grows with the
  code that ran and not with the program. It reads the table the host installed, which holds where each
  statement is written (`nvs_runtime::StmtLines`); with no table it is empty. The declared type is
  `array<array<int>>` because Novis's array is its map type.

What comes out is what existing tooling already reads: Clover and lcov for coverage, Callgrind and
speedscope for the profile. The trace is newline-delimited JSON, because no third-party tooling widely
consumes any trace format either. A coverage report opens in the CI dashboard a project already has
configured, and a profile opens in KCachegrind, Webgrind or speedscope unmodified, so there is no client
to write. The price is four exporters to keep correct instead of one, and a low barrier to entry is
worth it.

**There is one profiler, and it is deterministic.** The call probe pair times every call and attributes
exact self and inclusive time per function. `--profile=callgrind` writes the totals and
`--profile=speedscope` the timeline (`rule:observability/speedscope-timeline-export`), on `nvs run` and
`nvs test` alike. The probe adds its own cost to every call, so call-heavy code reads slower in a
profile than it runs, and the profiler's docs say so. Novis builds no sampling profiler.

- **Memory:** line and branch counters are bounded by the code a request touches, not by how often a
  loop iterates, so they live in the request's arena and are dropped with it. Trace and profile data
  are call-count-proportional, so they **stream to a sink** named when they start and hold nothing in
  memory, which leaves no buffer to bound with a new limit.

A spawned isolate starts from its parent's current flag word and may only narrow it. Its own data
never merges into the parent's live state; it comes back as fields on the result, by the same
copy-out rule a value, an error and a usage figure already use. Under coverage an isolate counts
into a fresh table of its parent's shape, so the parent's `coverage()` never lists a line only the
child ran; the host adds the child's table into a run table Novis code cannot read, which is what
`nvs test`'s report counts. A trace the child leaves running is closed at its end, and the result
carries its path and any write failure. A task of the same request is not an isolate and counts
into its request's table. No `Core` member reads these fields yet. Spends one counter per statement
per isolate in flight under coverage, freed at its join.

**Not on disk.** The probe bits exist (`crates/nvs-runtime/src/ctx/mod.rs:287`), and `nvs test` writes
lcov, Clover and Cobertura coverage (`crates/nvs-cli/src/main.rs:617`,
`crates/nvs-cli/src/coverage.rs`). `Core\Debug`'s seven members exist
(`crates/nvs-stdlib/src/debug.rs`), and a trace streams one line per returned call to its file
(`nvs_runtime::TraceSink`). A named profile sink is recorded and nothing writes to it yet. `nvs test` locates
its coverage tables, and `nvs run` installs a located table when `[debug] mode` has `coverage`, so
`coverage()` names the files and lines that ran. A spawned isolate's coverage table and closed
trace come back on its `Completion` (`nvs_runtime::host::Completion::coverage`). There is no
Callgrind or speedscope exporter, and `nvs run` has no probe flags.

**Declined:**
- **A sampling profiler.** It would interrupt compiled code on a timer and walk native frames on every
  target, a second profiler with platform code of its own, to remove a distortion the probe-based one
  states instead (`rule:observability/speedscope-timeline-export`).
- **A Novis-native coverage and profile format with a converter.** Both already have dominant
  consumers, and a conversion step is friction for every project that uses them.
- **Xdebug's own trace-file format.** Nothing beyond Xdebug's own tooling reads it widely, so
  matching it would buy no consumer.
- **Merging a child's coverage or trace into the parent's live state.** That needs mutable state
  crossing the arena boundary isolation exists to keep (`rule:security/isolate-shares-nothing`),
  and returning it on the result costs nothing new.
