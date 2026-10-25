`nvs run --profile=speedscope` and `nvs test --profile=speedscope` write the run's trace events, of
every kind `rule:observability/trace-events-carry-a-kind` names, as speedscope's evented-profile JSON:
an open and a close at a time on the run's one monotonic clock, which is what each trace event carries.
A call is a frame named for its callee. A `gc` pause, a `spawn` from its start to its join, and a
`query` or `http` a recorded run files are frames named for their kind, inside the call that was
running, so their time is never that call's self time. A time is never written before the one before
it on its timeline, which keeps a window that began before the latest event inside it.

**Each context is its own timeline**, an evented profile of the file's `profiles` list, because two
contexts' calls interleave and would not nest on one. The run's context has the first, named for the
program; under `nvs test` each test has one named for the test; and a spawned child's is named for
the timeline of the context that started it, with a number. The parent's `spawn` bar spans the
child's run on the parent's timeline, and the child's calls are on its own.

**Where the file goes.** `--profile=callgrind` writes `callgrind.out.<pid>`, the name KCachegrind
opens by default, and `--profile=speedscope` writes `profile.<pid>.speedscope.json`, both in the
working folder. `--profile-out <path>` names another file. The run prints the path on stderr when it
ends, and `Core\Debug::startProfile` writes speedscope to the path it was given.

It is one of the profiler's two exports. Novis has one profiler, the probe-based one
`rule:testing/debug-surface` describes, and `--profile=callgrind` is its other export: self and
inclusive time per function, for KCachegrind and Webgrind. The timeline shows the order things happened
in, and Callgrind shows where the time went in total. Both are written from the same probe events, so
the two files describe one run. Clover and lcov for coverage and the newline-delimited JSON trace stay
as `rule:testing/debug-surface` lists them.

The probe pair costs time at every call, so call-heavy code reads slower in a profile than it runs
unmeasured. The profiler's help text and its `about.md` say so in one sentence, because a reader who
does not know it will optimise the call overhead the probe added.

- **Memory:** the speedscope export streams to its file as events happen, the first timeline into the
  file and every other one into a part file beside it, copied in and deleted once the first closes.
  The Callgrind export adds each timeline's totals into the profiler's when it closes and writes the
  file at the end. A run holds one open entry per call frame in flight, one name per distinct
  function, and under Callgrind one total per function and per caller and callee pair, so the file
  grows with the run and the process does not.

**Declined:**
- **A sampling profiler beside the probe-based one.** It needs a timer that interrupts compiled code and
  a walk of native frames on every target, which is a second profiler with platform code of its own. The
  probe-based profiler already times every call, and its one distortion is stated where a reader meets it.
- **A Novis timeline format with a viewer of its own.** speedscope's format is open and its viewer is
  maintained elsewhere (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
