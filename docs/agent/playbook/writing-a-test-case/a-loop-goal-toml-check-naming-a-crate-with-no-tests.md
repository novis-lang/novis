- **A `loop-goal.toml` check naming a crate with no `tests/` directory is not misfiled; create the
  integration test there.** A package's ordinary `[dependencies]` are on the extern list of its test
  targets, so a new `crates/nvs-host/tests/limits.rs` can `use nvs_runtime::{Ctx, nvs_safepoint}`
  and drive compiled code with no `dev-dependencies` edit. The question is whether the crate can
  reach the thing the test asks about, never whether a test like it already lives there.
  [until: reviewed 2026-09-06]
