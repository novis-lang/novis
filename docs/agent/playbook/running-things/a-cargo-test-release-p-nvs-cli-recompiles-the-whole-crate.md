- **A `cargo test --release -p nvs-cli` recompiles the whole crate for two and a half minutes after
  any commit, and a cost guard run straight after that measures the link.**
  `crates/nvs-cli/build.rs` names the checkout's git `index` in a `cargo:rerun-if-changed`, so
  committing anything at all invalidates the release units, and the warm-start margin then read 1.8x
  where it names 4x on a commit that prints 13.2x once the box is idle. Build it with `--no-run`
  first and run the filter as a second call, and never read one red cost margin as a regression
  before that.
  [until: gone crates/nvs-cli/build.rs:join("index")]
