- **`Core\Config::restore` in a `.nvst` case can end it with a `FATAL`: the ceiling drops while the
  raised allowance is live.** A case that raises its `[limits]` ceiling, allocates and restores
  breaches at the next helper call; drop the ballast before restoring, since a request cannot
  un-allocate by lowering its own limit. A case may carry `--EXPECT--` and `--EXPECTF-ERROR--`
  together, so both the stdout before a `FATAL` and the `FATAL` line are pinnable.
  [until: gone crates/nvs-stdlib/src/config.rs:Core\Config::restore]
