`nvs dap` is wired into VS Code's existing debugger UI, and every feature below renders in a UI that
already exists — it appears only if the adapter reports the corresponding capability at `initialize`. The
adapter's capability list is therefore the debugger's scope, and it is this:

- **Conditional breakpoints, hit counts and logpoints** — `supportsConditionalBreakpoints`,
  `supportsHitConditionalBreakpoints`, `supportsLogPoints`. A logpoint that does not stop the program is
  the debugging most users actually do.
- **Exception filters** — `exceptionBreakpointFilters`, so "break on uncaught" and "break on thrown" are
  separate switches. `rule:errors/escalation-ladder`'s single `Throwable` channel is what makes this two
  filters and no more.
- **Stepping exclusions** — a `launch.json` glob list, so stepping does not descend into package code and
  a handled throw inside it does not stop the session.
- **Path mappings**, because the container case is the normal case: the file the adapter reports and the
  file in the editor differ whenever the program runs anywhere but the workspace root.
- **The value a function just returned**, in the variables pane after stepping out.
- **A `spawn`ed isolate is a DAP thread** — the standard presentation, needing no protocol extension. The
  *tree* of isolates would need one and is not built.

A fixture session exercises each of these, and `nvs dap` reports each capability at `initialize`.
