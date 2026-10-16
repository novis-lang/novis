- **A `default.toml` key whose `[limits.hard]` row has a fixed default fails two checks that
  disagree, until every block it appears in says bare `# default` with the ceiling's value.**
  `bun nv directives --check-template` wants one trailer per field across `[limits.hard]`,
  `[schedule.limits]` and `[app.limits.hard]`, and `crates/nvs-config/tests/defaults.rs` wants bare
  `# default` wherever its `row_key` finds a fixed row. Write `#request_body = "64M"  # default` in
  all three, and map a new block onto its rows in `row_key`. `until: gone crates/nvs-config/tests/defaults.rs:schedule.limits.`
