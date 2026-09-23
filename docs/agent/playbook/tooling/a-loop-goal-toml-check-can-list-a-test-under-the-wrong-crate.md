- **A loop-goal.toml check can list a test under the wrong crate because the rule it cites lives one
  layer up.** A check citing a rule that only `nvs-stdlib` implements, filed under `-p nvs-db`,
  reads as open work forever. Read the cited rule before writing the test, and split the check
  between the crates rather than moving it whole. [until: reviewed 2026-09-06]
