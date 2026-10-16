- **A `#[Core\Route]` method's return value is not the response body under `nvs serve <file>`: the
  script's top-level `echo` is.** The route table only tells the door which handler a request
  matched, so it can apply the CSRF and access checks. A file with a route and no `echo` answers
  `200` with an empty body. Write the output at the top level, as `LINKING` in
  `crates/nvs-cli/tests/live_config.rs` does, and see `benches/compare/scenarios/csrf/main.nvs`.
