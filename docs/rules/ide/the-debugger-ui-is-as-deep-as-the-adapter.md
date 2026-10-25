`nvs dap` is wired into VS Code's existing debugger UI, and every feature below renders in a UI that
already exists — it appears only if the adapter reports the corresponding capability at `initialize`. The
adapter's capability list is therefore the debugger's scope, and it is this:

- **Stepping over, into and out, one statement at a time** — `next`, `stepIn` and `stepOut` move a
  thread to its next stop site in the same frame, in any deeper frame, or in its caller. A step over an
  `await` stops at the next statement of the same frame once that task resumes, and the other tasks run
  while it waits. A step into a call through a `callable` lands in the function or closure it names. A
  step into a `Core` method steps over it, because it has no stop sites.
- **Conditional breakpoints, hit counts and logpoints** — `supportsConditionalBreakpoints`,
  `supportsHitConditionalBreakpoints`, `supportsLogPoints`. A logpoint that does not stop the program is
  the debugging most users actually do.
- **Exception filters** — `exceptionBreakpointFilters`, so "break on uncaught" and "break on thrown" are
  separate switches. `rule:errors/escalation-ladder`'s single `Throwable` channel is what makes this two
  filters and no more.
- **Stepping exclusions** — `skipFiles`, a `launch.json` glob list, so stepping does not descend into
  package code and a handled throw inside it does not stop the session.
- **Path mappings**, because the container case is the normal case: the file the adapter reports and the
  file in the editor differ whenever the program runs anywhere but the workspace root. `pathMappings`
  pairs an editor folder with the adapter's, as `{ "localRoot", "remoteRoot" }`, and every path in and
  out of the adapter is mapped: `program`, `cwd` and each breakpoint's file in, each frame's file out.
  The longest root that matches whole parts of a path wins, and a `skipFiles` glob is matched against
  the editor's path, since it is written in `launch.json` beside the rest.
- **The value a function just returned**, in the variables pane after stepping out. It is one frame's
  worth of state the adapter already has, and the UI renders it with no work of its own.
- **Every task of the debugged request is a DAP thread**, a `spawn`ed isolate's included, and a stop
  pauses all of them (`rule:testing/a-stop-pauses-the-whole-task-tree`). That is the standard
  presentation, needing no protocol extension. The *tree* of tasks would need one and is not built.

**A condition and a logpoint's `{…}` hole are expressions over the stopped frame's variables.** They
are type-checked against the frame's slot table
(`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`) when the breakpoint is set, and a
condition must be a `bool`. They may read variables, properties and array elements, and use literals,
operators and comparisons. They may not call a function or a method, assign, or `await`, so a
breakpoint never changes what the program does. A call is an error the editor shows: a condition or
hole that does not compile leaves its breakpoint unverified, with the compiler's message. A hit
condition is `N`, `>N`, `>=N`, `==N` or `%N`, counted per breakpoint.

**"Uncaught" is decided where the exception is thrown,** while every frame is still there to show. Under
the switch the compiler records which classes the `try` blocks around each statement catch, and each
frame record names the statement it is at, so the adapter knows at the throw whether any frame of that
task will catch it. A frame in a `skipFiles` file that catches it means the throw does not stop.

A fixture session exercises each of these, and `nvs dap` reports each capability at `initialize`.

- **Memory:** per stopped thread, one variable reference per expanded value, freed on resume; per
  breakpoint, its compiled condition and its hit count.

**Built:** every capability above, in `crates/nvs-cli/src/dap/`: steps through `nvs_host::debug`'s
`Debugger::step`, conditions and logpoints in `condition.rs` over `nvs_types::condition`'s checker,
the exception filters through `nvs_debug_throw`, the returned value in `stopped.rs`, and the path
mappings in `paths.rs`. `crates/nvs-cli/tests/dap.rs` is the fixture session.

**Declined:**
- **A condition that calls a function.** The call could write, throw or `await` at a breakpoint, and the
  program would then behave differently under the debugger than without it.
- **Deciding "uncaught" when the exception leaves the last frame.** The frames are gone by then, and the
  stop would show where the task ended instead of where it threw.
