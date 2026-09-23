- **A `loop-goal.toml` case can name a refusal whose checker half was never written, while the rule
  that specifies it reads as landed.** `rule:security/derived-codec-qualifiers` needs a tainted
  payload's receiving fields to declare the qualifier, but only its `secret` half has a diagnostic
  (`crates/nvs-diagnostics/src/lib.rs:966`), so the case named for the other half is a compiler
  slice. Grep the diagnostic registry for a rule's token before writing an `--EXPECTF-ERROR--` case:
  a fragment states what is decided, never what is on disk. [until: reviewed 2026-09-08]
