`nvs run --profile=speedscope` and `nvs test --profile=speedscope` write the run's trace events, of
every kind `rule:observability/trace-events-carry-a-kind` names, as speedscope's evented-profile JSON:
an open and a close at a time on the run's one monotonic clock, which is what each trace event carries.
speedscope opens the file as one scrollable timeline with every kind on it, and a spawned child's events
nest under its parent's `spawn` bar.

It is one of the profiler's two exports. Novis has one profiler, the probe-based one
`rule:testing/debug-surface` describes, and `--profile=callgrind` is its other export: self and
inclusive time per function, for KCachegrind and Webgrind. The timeline shows the order things happened
in, and Callgrind shows where the time went in total. Both are written from the same probe events, so
the two files describe one run. Clover and lcov for coverage and the newline-delimited JSON trace stay
as `rule:testing/debug-surface` lists them.

The probe pair costs time at every call, so call-heavy code reads slower in a profile than it runs
unmeasured. The profiler's help text and its `about.md` say so in one sentence, because a reader who
does not know it will optimise the call overhead the probe added.

- **Memory:** the export streams to its file as events close, through the sink
  `rule:testing/debug-surface` names. A run holds one open entry per call frame in flight and one name
  per distinct function, so the file grows with the run and the process does not.

**Declined:**
- **A sampling profiler beside the probe-based one.** It needs a timer that interrupts compiled code and
  a walk of native frames on every target, which is a second profiler with platform code of its own. The
  probe-based profiler already times every call, and its one distortion is stated where a reader meets it.
- **A Novis timeline format with a viewer of its own.** speedscope's format is open and its viewer is
  maintained elsewhere (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
