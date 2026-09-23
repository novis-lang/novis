- **A rule reading `env.signatures` from `nvs_types::lower` fires on every annotation: that pass
  runs twice and the first run has no table.** `signatures::collect_members` lowers every property
  type while the table is still an empty placeholder, so a roster check in `lower_property_key` sees
  every roster empty; only `env.symbols` is complete before both passes. Site such a rule where the
  table is real — `check.rs` re-lowers a property annotation, but a method parameter's annotation is
  lowered once, during collection. [until: reviewed 2026-09-06]
