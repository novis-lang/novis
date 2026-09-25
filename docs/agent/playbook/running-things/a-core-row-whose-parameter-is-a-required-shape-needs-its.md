- **A `Core` row whose parameter is a required *shape* needs its `$name` written in the spec's
  signature column, or two gates contradict each other.** `spec_registry_coverage.rs`'s
  `signature_names` reads a `{…}` with no `$name` as the *trailing options bag* and demands
  `names: [OPTIONS_NAME]`, while `registry.rs`'s
  `every_registry_row_names_one_parameter_per_positional_slot` forbids that very name on a positional
  slot — and a shape is positional. Amend the spec row to `{…} $settings` rather than the registry
  row, which is what that parser's own comment about `Core\Task::all` says a shape looks like.
  [until: gone crates/nvs-stdlib/src/registry.rs:every_registry_row_names_one_parameter_per_positional_slot]
