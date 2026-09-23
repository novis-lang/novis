- **A named case a goal owes may be pinning a hole, not a landed feature, and the failure mode is a
  *missing* line rather than a wrong one.** The shapes run, exit 0 and print something plausible, so
  freezing what `nvs run` printed pins the divergence as the expectation — a conformance case takes
  no `--ORACLE--`, so nothing else checks it against PHP. Run the shapes in a scratch
  `.agent-tmp/*.nvs`, write the same program as `.php`, and diff the two before filling in
  `--EXPECT--`. [until: reviewed 2026-09-06]
