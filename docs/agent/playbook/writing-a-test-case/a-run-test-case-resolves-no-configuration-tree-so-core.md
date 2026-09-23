- **A `--RUN-- test` case resolves no configuration tree, so `Core\Config` is empty inside every
  `#[Test]`.** `crates/nvs-cli/src/runner.rs` builds a bare `nvs_runtime::Ctx::stdout()` and only
  `nvs run` resolves `./nvs.toml`, so a `--FILE nvs.toml--` beside a `--RUN-- test` case is written
  and read by nobody, and `Core\Config::get` is `null`. Two sequential `spawn script` children do
  see the configuration:
  `tests/conformance/config/config-set-is-invisible-to-the-next-request.nvst`.
  [until: gone crates/nvs-cli/src/runner.rs:nvs_runtime::Ctx::stdout()]
