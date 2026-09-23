- **`nvs_config::value`'s `Unit::Duration` reads one unit, not a compound, so `"1m30s"` is refused
  where `"90s"` and a bare `90` agree.** `rule:types/duration-literal`'s literal is `1h30m`, so the
  two spellings look like one feature, and the refusal is `is not a duration` against a value the
  language accepts. Write a config bound as a bare count of the base unit, as `nvs_config::db`'s own
  bounds do; nothing in a block's module widens `crates/nvs-config/src/value.rs`'s parser.
  [until: reviewed 2026-09-06]
