- **A named case a goal owes may be pinning a hole, not a landed feature, and the failure mode is a
  *missing* line rather than a wrong one.** The shapes run, exit 0 and print something plausible, so
  freezing what `nvs run` printed pins the hole as the expectation, and nothing else checks the case
  against its rule. Write the output each rule the case cites says it prints first, then run the
  shapes in a scratch `.agent-tmp/*.nvs` and diff the two before filling in `--EXPECT--`.
  [until: gone crates/nvs-test/src/case.rs:--EXPECT--]
