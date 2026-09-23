- **`nvs run`'s entry point has a second caller, and nothing near it says so.** `run_run` in
  `crates/nvs-cli/src/main.rs` is also how `bundle::run` starts a bundled program, so a new parameter
  breaks a call site in `crates/nvs-cli/src/bundle.rs` that no anchor in the `Command::Run` region
  points at — and what to pass there is a decision rather than a mechanical fill-in. Grep
  `super::run_run` before changing that signature, and answer for the bundle first: nothing stands in
  front of it to have handed it anything, so its answer is almost always the absent one.
  [until: gone crates/nvs-cli/src/bundle.rs:super::run_run]
