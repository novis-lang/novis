- **A `.nvst` case can write its own `nvs.toml`, so a capability-gated member's granted error paths
  must be asserted.** `--FILE nvs.toml--` lands in the case's directory, where
  `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 2 reads it, so
  `[capabilities.script] spawn = true` grants the run and no `nvs.toml` grants nothing. Never repair
  the gate with `OWED_A_CASE` or a false `DECLARATION`; it reads throws sited in `nvs-stdlib`.
  [until: reviewed 2026-09-06]
